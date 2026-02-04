use crate::launcher::domain::{Library, VersionManifest};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use reqwest::StatusCode;

pub struct FabricInstaller;
pub type ProgressCallback = Arc<dyn Fn(f32, String) + Send + Sync>;

const PROFILE_START: f32 = 0.1;
const PROFILE_END: f32 = 0.2;
const LIBS_START: f32 = 0.2;
const LIBS_END: f32 = 0.95;

#[derive(Debug, Deserialize)]
struct LoaderVersionResponse {
    loader: LoaderVersion,
}

#[derive(Debug, Deserialize)]
struct LoaderVersion {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize)]
struct FabricProfile {
    id: String,
    #[serde(default)]
    libraries: Vec<Library>,
}

impl FabricInstaller {
    pub async fn is_supported(mc_version: &str) -> Result<bool> {
        let url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            urlencoding::encode(mc_version)
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .context("Failed to fetch Fabric loader versions")?;

        if resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::BAD_REQUEST {
            return Ok(false);
        }
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Fabric meta returned an error: {}",
                resp.status()
            ));
        }

        let versions: Vec<LoaderVersionResponse> = resp
            .json()
            .await
            .context("Failed to parse Fabric loader versions")?;

        Ok(!versions.is_empty())
    }

    pub async fn ensure_fabric_profile(
        minecraft_dir: &Path,
        mc_version: &str,
        on_progress: Option<ProgressCallback>,
    ) -> Result<String> {
        let versions_dir = minecraft_dir.join("versions");
        Self::report(&on_progress, 0.02, "Checking Fabric installation");

        if let Some(mut existing) = Self::find_installed_profile(&versions_dir, mc_version)? {
            if !Self::is_profile_complete(&versions_dir, &existing) {
                Self::report(&on_progress, 0.08, "Repairing Fabric profile");
                let loader_version = Self::parse_loader_version(&existing, mc_version)
                    .unwrap_or_else(|| "unknown".to_string());

                let loader_version = if loader_version == "unknown" {
                    Self::report(&on_progress, 0.12, "Fetching latest Fabric loader");
                    Self::fetch_latest_loader_version(mc_version).await?
                } else {
                    loader_version
                };

                let version_id = format!("fabric-loader-{}-{}", loader_version, mc_version);
                Self::report(&on_progress, PROFILE_START, "Downloading Fabric profile");
                Self::download_and_extract_profile_zip(
                    &versions_dir,
                    mc_version,
                    &loader_version,
                    on_progress.clone(),
                )
                .await?;
                existing = version_id;
            }

            if !Self::is_profile_complete(&versions_dir, &existing) {
                return Err(anyhow::anyhow!(
                    "Fabric profile is incomplete after install: {}",
                    existing
                ));
            }

            Self::report(&on_progress, LIBS_START, "Preparing Fabric libraries");
            Self::ensure_fabric_libraries(minecraft_dir, &existing, on_progress.clone()).await?;
            return Ok(existing);
        }

        Self::report(&on_progress, 0.1, "Fetching latest Fabric loader");
        let loader_version = Self::fetch_latest_loader_version(mc_version).await?;
        let version_id = format!("fabric-loader-{}-{}", loader_version, mc_version);

        Self::report(&on_progress, PROFILE_START, "Downloading Fabric profile");
        Self::download_and_extract_profile_zip(
            &versions_dir,
            mc_version,
            &loader_version,
            on_progress.clone(),
        )
            .await?;
        if !Self::is_profile_complete(&versions_dir, &version_id) {
            return Err(anyhow::anyhow!(
                "Fabric profile is incomplete after install: {}",
                version_id
            ));
        }
        Self::report(&on_progress, LIBS_START, "Preparing Fabric libraries");
        Self::ensure_fabric_libraries(minecraft_dir, &version_id, on_progress.clone()).await?;
        Self::report(&on_progress, 1.0, "Fabric ready");

        Ok(version_id)
    }

    fn find_installed_profile(
        versions_dir: &Path,
        mc_version: &str,
    ) -> Result<Option<String>> {
        if !versions_dir.exists() {
            return Ok(None);
        }

        let suffix = format!("-{}", mc_version);
        let mut best: Option<(std::time::SystemTime, String)> = None;

        for entry in std::fs::read_dir(versions_dir).context("Failed to read versions dir")? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.path().is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("fabric-loader-") || !name.ends_with(&suffix) {
                continue;
            }

            let json_path = entry.path().join(format!("{}.json", name));
            if !json_path.exists() {
                continue;
            }

            let modified = std::fs::metadata(&json_path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            match &best {
                Some((best_time, _)) if *best_time >= modified => {}
                _ => best = Some((modified, name.clone())),
            }
        }

        Ok(best.map(|(_, name)| name))
    }

    async fn fetch_latest_loader_version(mc_version: &str) -> Result<String> {
        let url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            urlencoding::encode(mc_version)
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .context("Failed to fetch Fabric loader versions")?
            .error_for_status()
            .context("Fabric meta returned an error")?;

        let versions: Vec<LoaderVersionResponse> = resp
            .json()
            .await
            .context("Failed to parse Fabric loader versions")?;

        let latest = versions
            .iter()
            .find(|v| v.loader.stable)
            .or_else(|| versions.first())
            .ok_or_else(|| anyhow::anyhow!("No Fabric loader versions found"))?;

        Ok(latest.loader.version.clone())
    }

    async fn download_and_extract_profile_zip(
        versions_dir: &Path,
        mc_version: &str,
        loader_version: &str,
        on_progress: Option<ProgressCallback>,
    ) -> Result<()> {
        let url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/zip",
            urlencoding::encode(mc_version),
            urlencoding::encode(loader_version)
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .context("Failed to download Fabric profile zip")?
            .error_for_status()
            .context("Fabric profile zip request failed")?;

        let total = resp.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut bytes: Vec<u8> = Vec::new();

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read Fabric profile zip chunk")?;
            downloaded += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);

            let progress = if total > 0 {
                downloaded as f32 / total as f32
            } else {
                0.0
            };
            let overall = PROFILE_START + (PROFILE_END - PROFILE_START) * progress;
            let status = if total > 0 {
                format!(
                    "Downloading Fabric profile ({:.1}/{:.1} MB)",
                    downloaded as f32 / 1024.0 / 1024.0,
                    total as f32 / 1024.0 / 1024.0
                )
            } else {
                format!(
                    "Downloading Fabric profile ({:.1} MB)",
                    downloaded as f32 / 1024.0 / 1024.0
                )
            };
            Self::report(&on_progress, overall, status);
        }

        if bytes.is_empty() {
            return Err(anyhow::anyhow!("Fabric profile zip is empty"));
        }

        if !versions_dir.exists() {
            std::fs::create_dir_all(versions_dir)
                .context("Failed to create versions directory")?;
        }

        Self::extract_zip(&bytes, versions_dir).context("Failed to extract Fabric profile zip")?;
        Self::report(&on_progress, PROFILE_END, "Fabric profile ready");

        Ok(())
    }

    fn extract_zip(bytes: &[u8], target_dir: &Path) -> Result<()> {
        let reader = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).context("Invalid zip archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
                continue;
            }

            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        Ok(())
    }

    async fn ensure_fabric_libraries(
        minecraft_dir: &Path,
        version_id: &str,
        on_progress: Option<ProgressCallback>,
    ) -> Result<()> {
        let versions_dir = minecraft_dir.join("versions");
        let profile_path = versions_dir
            .join(version_id)
            .join(format!("{}.json", version_id));

        let content = std::fs::read_to_string(&profile_path)
            .with_context(|| format!("Failed to read Fabric profile: {}", profile_path.display()))?;

        let profile: FabricProfile =
            serde_json::from_str(&content).context("Failed to parse Fabric profile JSON")?;

        let libraries_dir = minecraft_dir.join("libraries");
        let client = reqwest::Client::new();

        let downloads = Self::collect_library_downloads(&profile, &libraries_dir)?;
        let total_bytes: u64 = downloads.iter().filter_map(|d| d.size).sum();
        let total_downloads = downloads.len();

        if total_downloads == 0 {
            Self::report(&on_progress, LIBS_END, "Fabric libraries already present");
            return Ok(());
        }

        let mut tracker = DownloadTracker {
            total_bytes,
            downloaded_bytes: 0,
        };

        for (index, download) in downloads.iter().enumerate() {
            let status = format!(
                "Downloading library {}/{}",
                index + 1,
                total_downloads
            );
            Self::report(&on_progress, Self::overall_progress(&tracker), status);

            Self::download_to_path(
                &client,
                &download.url,
                &download.dest,
                download.size,
                &mut tracker,
                on_progress.clone(),
            )
            .await?;
        }

        Self::report(&on_progress, LIBS_END, "Fabric libraries ready");
        Ok(())
    }

    async fn download_to_path(
        client: &reqwest::Client,
        url: &str,
        dest: &Path,
        size_hint: Option<u64>,
        tracker: &mut DownloadTracker,
        on_progress: Option<ProgressCallback>,
    ) -> Result<()> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create library directory")?;
        }

        let resp = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to download {}", url))?
            .error_for_status()
            .context("Library download failed")?;

        if size_hint.unwrap_or(0) == 0 {
            if let Some(len) = resp.content_length() {
                tracker.total_bytes += len;
            }
        }

        let mut file = tokio::fs::File::create(dest)
            .await
            .with_context(|| format!("Failed to create {}", dest.display()))?;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read library chunk")?;
            file.write_all(&chunk).await?;
            tracker.downloaded_bytes += chunk.len() as u64;
            Self::report(&on_progress, Self::overall_progress(tracker), "Downloading libraries");
        }

        Self::report(&on_progress, Self::overall_progress(tracker), "Library downloaded");
        Ok(())
    }

    fn is_profile_complete(versions_dir: &Path, version_id: &str) -> bool {
        let json_path = versions_dir
            .join(version_id)
            .join(format!("{}.json", version_id));

        let json_ok = json_path
            .metadata()
            .map(|m| m.len() > 0)
            .unwrap_or(false);

        json_ok
    }

    fn parse_loader_version(version_id: &str, mc_version: &str) -> Option<String> {
        let prefix = "fabric-loader-";
        let suffix = format!("-{}", mc_version);
        if !version_id.starts_with(prefix) || !version_id.ends_with(&suffix) {
            return None;
        }

        let trimmed = version_id.strip_prefix(prefix)?;
        let loader = trimmed.strip_suffix(&suffix)?;
        if loader.is_empty() {
            None
        } else {
            Some(loader.to_string())
        }
    }

    fn report(on_progress: &Option<ProgressCallback>, progress: f32, status: impl Into<String>) {
        if let Some(cb) = on_progress {
            let clamped = progress.clamp(0.0, 1.0);
            cb(clamped, status.into());
        }
    }

    fn overall_progress(tracker: &DownloadTracker) -> f32 {
        if tracker.total_bytes == 0 {
            return LIBS_START;
        }
        let frac = tracker.downloaded_bytes as f32 / tracker.total_bytes as f32;
        LIBS_START + (LIBS_END - LIBS_START) * frac.min(1.0)
    }
}

struct DownloadTracker {
    total_bytes: u64,
    downloaded_bytes: u64,
}

struct LibraryDownload {
    url: String,
    dest: PathBuf,
    size: Option<u64>,
}

impl FabricInstaller {
    fn collect_library_downloads(
        profile: &FabricProfile,
        libraries_dir: &Path,
    ) -> Result<Vec<LibraryDownload>> {
        let mut downloads = Vec::new();

        for library in &profile.libraries {
            if !VersionManifest::should_include_library_for_current_os(library) {
                continue;
            }

            if let Some(downloads_info) = &library.downloads {
                if let Some(artifact) = &downloads_info.artifact {
                    let dest = libraries_dir.join(&artifact.path);
                    if dest.exists() {
                        continue;
                    }
                    downloads.push(LibraryDownload {
                        url: artifact.url.clone(),
                        dest,
                        size: Some(artifact.size),
                    });
                    continue;
                }
            }

            if let Some(base_url) = &library.url {
                let artifact = MavenArtifact::parse(&library.name)?;
                let rel_path = artifact.relative_path();
                let rel_path_str = rel_path.to_string_lossy();
                let url = format!("{}/{}", base_url.trim_end_matches('/'), rel_path_str);
                let dest = libraries_dir.join(&rel_path);

                if dest.exists() {
                    continue;
                }

                downloads.push(LibraryDownload {
                    url,
                    dest,
                    size: None,
                });
            }
        }

        Ok(downloads)
    }
}

#[derive(Debug)]
struct MavenArtifact {
    group: String,
    artifact: String,
    version: String,
    classifier: Option<String>,
    extension: String,
}

impl MavenArtifact {
    fn parse(name: &str) -> Result<Self> {
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid maven coordinate: {}", name));
        }

        let group = parts[0].to_string();
        let artifact = parts[1].to_string();
        let version = parts[2].to_string();
        let classifier = if parts.len() >= 4 {
            Some(parts[3].to_string())
        } else {
            None
        };
        let extension = if parts.len() >= 5 {
            parts[4].to_string()
        } else {
            "jar".to_string()
        };

        Ok(Self {
            group,
            artifact,
            version,
            classifier,
            extension,
        })
    }

    fn relative_path(&self) -> PathBuf {
        let group_path = self.group.replace('.', "/");
        let file_name = match &self.classifier {
            Some(classifier) => format!(
                "{}-{}-{}.{}",
                self.artifact, self.version, classifier, self.extension
            ),
            None => format!("{}-{}.{}", self.artifact, self.version, self.extension),
        };

        PathBuf::from(group_path)
            .join(&self.artifact)
            .join(&self.version)
            .join(file_name)
    }
}
