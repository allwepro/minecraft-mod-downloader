use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct JavaVersion {
    pub version: u32, // 8, 17, 21
    pub release_name: String,
    pub download_url: String,
    pub filename: String,
    pub size: u64,
}

pub struct JavaDownloadService;

#[derive(Deserialize, Debug)]
struct AdoptiumRelease {
    binary: AdoptiumBinary,
    release_name: String,
}

#[derive(Deserialize, Debug)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Deserialize, Debug)]
struct AdoptiumPackage {
    link: String,
    name: String,
    size: u64,
}

impl JavaDownloadService {
    pub async fn fetch_available_versions() -> Result<Vec<JavaVersion>> {
        let versions = [8, 17, 21];
        let mut available = Vec::new();
        let client = reqwest::Client::new();

        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else {
            "linux"
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x64" // Fallback
        };

        for version in versions {
            let url = format!(
                "https://api.adoptium.net/v3/assets/latest/{}/hotspot?os={}&architecture={}&image_type=jdk",
                version, os, arch
            );

            match client.get(&url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let releases: Vec<AdoptiumRelease> = resp.json().await.unwrap_or_default();
                        if let Some(release) = releases.first() {
                            available.push(JavaVersion {
                                version,
                                release_name: release.release_name.clone(),
                                download_url: release.binary.package.link.clone(),
                                filename: release.binary.package.name.clone(),
                                size: release.binary.package.size,
                            });
                        }
                    } else {
                        log::warn!("Failed to fetch Java {}: Status {}", version, resp.status());
                    }
                }
                Err(e) => {
                    log::warn!("Failed to fetch Java {}: {}", version, e);
                }
            }
        }

        Ok(available)
    }

    pub async fn download_and_extract(
        version: &JavaVersion,
        target_dir: PathBuf,
        on_progress: impl Fn(f32, &str) + Send + 'static,
    ) -> Result<PathBuf> {
        on_progress(0.0, "Starting download...");

        let client = reqwest::Client::new();
        let mut response = client
            .get(&version.download_url)
            .send()
            .await
            .context("Failed to start download request")?;

        let total_size = response.content_length().unwrap_or(version.size);
        let mut downloaded: u64 = 0;

        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(&version.filename);
        let mut dest_file = File::create(&temp_file_path).context("Failed to create temp file")?;

        while let Some(chunk) = response.chunk().await.context("Error downloading chunk")? {
            dest_file
                .write_all(&chunk)
                .context("Error writing to file")?;
            downloaded += chunk.len() as u64;
            let progress = downloaded as f32 / total_size as f32;
            let downloaded_mb = downloaded as f32 / 1024.0 / 1024.0;
            let total_mb = total_size as f32 / 1024.0 / 1024.0;
            on_progress(
                progress,
                &format!("Downloading... ({:.1}/{:.1} MB)", downloaded_mb, total_mb),
            );
        }

        on_progress(1.0, "Extracting...");

        // Ensure target directory exists
        if !target_dir.exists() {
            std::fs::create_dir_all(&target_dir).context("Failed to create target directory")?;
        }

        let java_home = Self::extract_archive(&temp_file_path, &target_dir)
            .context("Failed to extract archive")?;

        // Cleanup
        let _ = std::fs::remove_file(temp_file_path);

        on_progress(1.0, "Complete!");
        Ok(java_home)
    }

    fn extract_archive(archive_path: &Path, target_dir: &Path) -> Result<PathBuf> {
        let file = File::open(archive_path)?;
        let mut extracted_root: Option<PathBuf> = None;

        if archive_path.extension().and_then(|s| s.to_str()) == Some("zip") {
            let mut archive = zip::ZipArchive::new(file)?;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => target_dir.join(path),
                    None => continue,
                };

                // Capture the root directory of the extraction
                if i == 0 {
                    if let Some(comp) = outpath
                        .strip_prefix(target_dir)
                        .ok()
                        .and_then(|p| p.components().next())
                    {
                        extracted_root = Some(target_dir.join(comp.as_os_str()));
                    }
                }

                if file.name().ends_with('/') {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }
        } else {
            // Assume tar.gz for non-zip (mac/linux)
            let tar = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(tar);

            // We need to inspect entries to find root, but tar reader consumes stream.
            // Simplified: extract all, then find directory in target_dir.
            archive.unpack(target_dir)?;

            // Heuristic to find the extracted folder
            if let Ok(entries) = std::fs::read_dir(target_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Expect something like "jdk-17.0.1+12"
                        if name.contains("jdk") || name.contains("jre") {
                            extracted_root = Some(entry.path());
                            break;
                        }
                    }
                }
            }
        }

        extracted_root.ok_or_else(|| anyhow::anyhow!("Could not determine extracted Java root"))
    }
}
