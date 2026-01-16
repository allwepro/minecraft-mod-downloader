use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadMetadataEntry {
    pub file: String,
    pub version: String,
    pub downloaded_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DownloadMetadata {
    pub mods: HashMap<String, DownloadMetadataEntry>,
}

impl DownloadMetadata {
    pub fn new() -> Self {
        Self {
            mods: HashMap::new(),
        }
    }

    pub fn get_entry(&self, mod_id: &str) -> Option<&DownloadMetadataEntry> {
        self.mods.get(mod_id)
    }

    pub fn update_entry(&mut self, mod_id: String, file: String, version: String) {
        self.mods.insert(
            mod_id,
            DownloadMetadataEntry {
                file,
                version,
                downloaded_at: Utc::now(),
            },
        );
    }

    pub fn remove_entry(&mut self, mod_id: &str) {
        self.mods.remove(mod_id);
    }

    pub fn validate_and_cleanup(&mut self, download_dir: &Path) {
        let mut to_remove = Vec::new();

        for (mod_id, entry) in &self.mods {
            let file_path = download_dir.join(&entry.file);
            if !file_path.exists() {
                to_remove.push(mod_id.clone());
            }
        }

        for mod_id in to_remove {
            log::debug!("Removing stale metadata entry for {mod_id} (file no longer exists)");
            self.mods.remove(&mod_id);
        }
    }
}

pub async fn read_download_metadata(download_dir: &Path) -> Result<DownloadMetadata> {
    let metadata_path = download_dir.join(".mcd.json");

    if !metadata_path.exists() {
        return Ok(DownloadMetadata::new());
    }

    let content = tokio::fs::read_to_string(&metadata_path).await?;
    let metadata: DownloadMetadata = serde_json::from_str(&content)?;

    Ok(metadata)
}

pub async fn write_download_metadata(
    download_dir: &Path,
    metadata: &DownloadMetadata,
) -> Result<()> {
    tokio::fs::create_dir_all(download_dir).await?;

    let metadata_path = download_dir.join(".mcd.json");
    let temp_path = download_dir.join(".mcd.json.tmp");

    let json_content = serde_json::to_string_pretty(metadata)?;
    tokio::fs::write(&temp_path, json_content).await?;

    tokio::fs::rename(&temp_path, &metadata_path).await?;

    Ok(())
}

pub async fn update_metadata_entry(
    download_dir: &Path,
    mod_id: String,
    filename: String,
    version: String,
) -> Result<()> {
    let mut metadata = read_download_metadata(download_dir).await?;
    metadata.update_entry(mod_id, filename, version);
    write_download_metadata(download_dir, &metadata).await?;
    Ok(())
}

pub async fn remove_metadata_entry(download_dir: &Path, mod_id: &str) -> Result<()> {
    let mut metadata = read_download_metadata(download_dir).await?;
    metadata.remove_entry(mod_id);
    write_download_metadata(download_dir, &metadata).await?;
    Ok(())
}
