use crate::launcher::domain::{Library, VersionManifest};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

pub struct MinecraftDownloadService;

pub type ProgressCallback = Arc<dyn Fn(f32, String) + Send + Sync>;

const GLOBAL_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

const STEP_MANIFEST: f32 = 0.05;
const STEP_VERSION_JSON: f32 = 0.1;
const STEP_CLIENT: f32 = 0.3;
const STEP_LIBRARIES: f32 = 0.7;
const STEP_ASSETS: f32 = 0.95;

#[derive(Debug, Clone)]
pub struct MinecraftVersionInfo {
    pub id: String,
    pub version_type: String,
    pub release_time: String,
}

#[derive(Debug, Deserialize)]
struct GlobalVersionManifest {
    versions: Vec<GlobalVersionEntry>,
}

#[derive(Debug, Deserialize)]
struct GlobalVersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct AssetIndexFile {
    objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

impl MinecraftDownloadService {
    pub async fn fetch_available_versions(
        minecraft_dir: &Path,
    ) -> Result<Vec<MinecraftVersionInfo>> {
        let manifest = Self::load_global_manifest(minecraft_dir).await?;

        let mut versions: Vec<MinecraftVersionInfo> = manifest
            .versions
            .into_iter()
            .map(|v| MinecraftVersionInfo {
                id: v.id,
                version_type: v.version_type,
                release_time: v.release_time,
            })
            .collect();

        versions.sort_by(|a, b| b.release_time.cmp(&a.release_time));
        Ok(versions)
    }

    pub async fn install_version(
        minecraft_dir: &Path,
        version_id: &str,
        on_progress: Option<ProgressCallback>,
    ) -> Result<()> {
        Self::install_version_inner(minecraft_dir, version_id, on_progress, 0).await
    }

    async fn install_version_inner(
        minecraft_dir: &Path,
        version_id: &str,
        on_progress: Option<ProgressCallback>,
        depth: u8,
    ) -> Result<()> {
        if depth > 3 {
            return Err(anyhow::anyhow!("Version inheritance too deep"));
        }

        Self::report(&on_progress, 0.0, "Loading version manifest");
        let manifest = Self::load_global_manifest(minecraft_dir).await?;
        let version_entry = manifest
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| anyhow::anyhow!("Minecraft version not found: {}", version_id))?;

        Self::report(&on_progress, STEP_MANIFEST, "Downloading version metadata");
        let version_json_path =
            Self::ensure_version_json(minecraft_dir, version_id, &version_entry.url, &on_progress)
                .await?;

        Self::report(&on_progress, STEP_VERSION_JSON, "Reading version metadata");
        let version_manifest = VersionManifest::from_file(&version_json_path)
            .context("Failed to parse version metadata")?;

        if let Some(parent) = &version_manifest.inherits_from {
            if parent != version_id {
                Box::pin(Self::install_version_inner(
                    minecraft_dir,
                    parent,
                    on_progress.clone(),
                    depth + 1,
                ))
                .await?;
            }
        }

        if let Some(downloads) = &version_manifest.downloads {
            if let Some(client) = &downloads.client {
                let client_jar = minecraft_dir
                    .join("versions")
                    .join(version_id)
                    .join(format!("{}.jar", version_id));

                Self::report(&on_progress, STEP_VERSION_JSON, "Downloading client jar");
                Self::download_file(
                    &client.url,
                    &client_jar,
                    Some(client.size),
                    &on_progress,
                    STEP_VERSION_JSON,
                    STEP_CLIENT - STEP_VERSION_JSON,
                    "Downloading client jar",
                )
                .await?;
            }
        }

        let libraries_dir = minecraft_dir.join("libraries");
        Self::download_libraries(&libraries_dir, &version_manifest.libraries, &on_progress).await?;

        if let Some(asset_index) = &version_manifest.asset_index {
            Self::download_assets(minecraft_dir, asset_index, &on_progress).await?;
        }

        Self::report(&on_progress, 1.0, "Minecraft ready");
        Ok(())
    }

    async fn load_global_manifest(minecraft_dir: &Path) -> Result<GlobalVersionManifest> {
        let manifest_path = minecraft_dir.join("version_manifest_v2.json");

        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)
                .context("Failed to read local version manifest")?;
            let manifest: GlobalVersionManifest =
                serde_json::from_str(&content).context("Failed to parse version manifest")?;
            return Ok(manifest);
        }

        let client = reqwest::Client::new();
        let resp = client
            .get(GLOBAL_MANIFEST_URL)
            .send()
            .await
            .context("Failed to download global version manifest")?
            .error_for_status()
            .context("Global version manifest request failed")?;

        let bytes = resp
            .bytes()
            .await
            .context("Failed to read global manifest bytes")?;

        tokio::fs::create_dir_all(minecraft_dir)
            .await
            .context("Failed to create minecraft directory")?;
        tokio::fs::write(&manifest_path, &bytes)
            .await
            .context("Failed to write local version manifest")?;

        let manifest: GlobalVersionManifest =
            serde_json::from_slice(&bytes).context("Failed to parse version manifest")?;
        Ok(manifest)
    }

    async fn ensure_version_json(
        minecraft_dir: &Path,
        version_id: &str,
        url: &str,
        on_progress: &Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        let version_dir = minecraft_dir.join("versions").join(version_id);
        let version_json_path = version_dir.join(format!("{}.json", version_id));

        if version_json_path.exists() {
            return Ok(version_json_path);
        }

        tokio::fs::create_dir_all(&version_dir)
            .await
            .context("Failed to create version directory")?;

        Self::report(on_progress, STEP_MANIFEST, "Downloading version JSON");
        Self::download_file(
            url,
            &version_json_path,
            None,
            on_progress,
            STEP_MANIFEST,
            STEP_VERSION_JSON - STEP_MANIFEST,
            "Downloading version JSON",
        )
        .await?;

        Ok(version_json_path)
    }

    async fn download_libraries(
        libraries_dir: &Path,
        libraries: &[Library],
        on_progress: &Option<ProgressCallback>,
    ) -> Result<()> {
        let downloads = Self::collect_library_downloads(libraries, libraries_dir)?;
        if downloads.is_empty() {
            return Ok(());
        }

        let total_bytes: u64 = downloads.iter().filter_map(|d| d.size).sum();
        let total_count = downloads.len();
        let mut tracker = ByteTracker {
            total_bytes,
            downloaded_bytes: 0,
        };

        for (index, item) in downloads.iter().enumerate() {
            Self::download_file_tracked(
                &item.url,
                &item.dest,
                item.size,
                on_progress,
                STEP_CLIENT,
                STEP_LIBRARIES - STEP_CLIENT,
                "Downloading libraries",
                &mut tracker,
                index,
                total_count,
            )
            .await?;
        }

        Ok(())
    }

    async fn download_assets(
        minecraft_dir: &Path,
        asset_index: &crate::launcher::domain::AssetIndex,
        on_progress: &Option<ProgressCallback>,
    ) -> Result<()> {
        let assets_dir = minecraft_dir.join("assets");
        let index_dir = assets_dir.join("indexes");
        let objects_dir = assets_dir.join("objects");

        tokio::fs::create_dir_all(&index_dir)
            .await
            .context("Failed to create assets index directory")?;
        tokio::fs::create_dir_all(&objects_dir)
            .await
            .context("Failed to create assets objects directory")?;

        let index_path = index_dir.join(format!("{}.json", asset_index.id));
        if !index_path.exists() {
            Self::download_file(
                &asset_index.url,
                &index_path,
                Some(asset_index.size),
                on_progress,
                STEP_LIBRARIES,
                STEP_ASSETS - STEP_LIBRARIES,
                "Downloading asset index",
            )
            .await?;
        }

        let index_content = tokio::fs::read_to_string(&index_path)
            .await
            .context("Failed to read asset index")?;
        let asset_index_file: AssetIndexFile =
            serde_json::from_str(&index_content).context("Failed to parse asset index")?;

        let mut assets = Vec::new();
        let mut total_bytes = 0u64;

        for object in asset_index_file.objects.values() {
            let hash = &object.hash;
            let prefix = &hash[0..2];
            let dest = objects_dir.join(prefix).join(hash);
            if dest.exists() {
                continue;
            }

            let url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                prefix, hash
            );
            assets.push(LibraryDownload {
                url,
                dest,
                size: Some(object.size),
            });
            total_bytes += object.size;
        }

        if assets.is_empty() {
            return Ok(());
        }

        let total_count = assets.len();
        let mut tracker = ByteTracker {
            total_bytes,
            downloaded_bytes: 0,
        };

        for (index, item) in assets.iter().enumerate() {
            Self::download_file_tracked(
                &item.url,
                &item.dest,
                item.size,
                on_progress,
                STEP_LIBRARIES,
                STEP_ASSETS - STEP_LIBRARIES,
                "Downloading assets",
                &mut tracker,
                index,
                total_count,
            )
            .await?;
        }

        Ok(())
    }

    fn collect_library_downloads(
        libraries: &[Library],
        libraries_dir: &Path,
    ) -> Result<Vec<LibraryDownload>> {
        let mut downloads = Vec::new();

        for library in libraries {
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

    async fn download_file(
        url: &str,
        dest: &Path,
        size_hint: Option<u64>,
        on_progress: &Option<ProgressCallback>,
        base: f32,
        span: f32,
        status: &str,
    ) -> Result<()> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create download directory")?;
        }

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to download {}", url))?
            .error_for_status()
            .context("Download request failed")?;

        let total = resp.content_length().or(size_hint).unwrap_or(0);
        let mut downloaded = 0u64;

        let tmp_path = dest.with_extension("tmp");
        let mut file = tokio::fs::File::create(&tmp_path)
            .await
            .with_context(|| format!("Failed to create {}", tmp_path.display()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read download chunk")?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if total > 0 {
                let progress = downloaded as f32 / total as f32;
                Self::report(
                    on_progress,
                    base + span * progress.min(1.0),
                    status.to_string(),
                );
            } else {
                Self::report(on_progress, base, status.to_string());
            }
        }

        tokio::fs::rename(&tmp_path, dest)
            .await
            .with_context(|| format!("Failed to move {}", dest.display()))?;

        Ok(())
    }

    async fn download_file_tracked(
        url: &str,
        dest: &Path,
        size_hint: Option<u64>,
        on_progress: &Option<ProgressCallback>,
        base: f32,
        span: f32,
        status: &str,
        tracker: &mut ByteTracker,
        index: usize,
        total_count: usize,
    ) -> Result<()> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create download directory")?;
        }

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to download {}", url))?
            .error_for_status()
            .context("Download request failed")?;

        let total = resp.content_length().or(size_hint).unwrap_or(0);
        let mut downloaded = 0u64;

        let tmp_path = dest.with_extension("tmp");
        let mut file = tokio::fs::File::create(&tmp_path)
            .await
            .with_context(|| format!("Failed to create {}", tmp_path.display()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read download chunk")?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            tracker.downloaded_bytes += chunk.len() as u64;

            let progress = if tracker.total_bytes > 0 {
                tracker.downloaded_bytes as f32 / tracker.total_bytes as f32
            } else if total_count > 0 {
                (index as f32 / total_count as f32).min(1.0)
            } else {
                0.0
            };

            Self::report(on_progress, base + span * progress, status.to_string());
        }

        if total > 0 && downloaded != total {
            // If the server didn't match content length, just proceed but log progress as-is.
            Self::report(on_progress, base + span, status.to_string());
        }

        tokio::fs::rename(&tmp_path, dest)
            .await
            .with_context(|| format!("Failed to move {}", dest.display()))?;

        Ok(())
    }

    fn report(on_progress: &Option<ProgressCallback>, progress: f32, status: impl Into<String>) {
        if let Some(cb) = on_progress {
            let clamped = progress.clamp(0.0, 1.0);
            cb(clamped, status.into());
        }
    }
}

struct LibraryDownload {
    url: String,
    dest: PathBuf,
    size: Option<u64>,
}

struct ByteTracker {
    total_bytes: u64,
    downloaded_bytes: u64,
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
