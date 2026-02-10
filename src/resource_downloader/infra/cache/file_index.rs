use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileIndexEntry {
    pub hash: String,
    pub size: u64,
    pub modified: u64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct FileIndexCache {
    pub entries: HashMap<String, FileIndexEntry>,
}

impl FileIndexCache {
    pub async fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).await?;
        let cache = serde_json::from_str(&content).unwrap_or_default();
        Ok(cache)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    pub fn get(&self, path: &Path) -> Option<&FileIndexEntry> {
        self.entries.get(path.to_string_lossy().as_ref())
    }

    pub fn insert(&mut self, path: PathBuf, entry: FileIndexEntry) {
        self.entries
            .insert(path.to_string_lossy().to_string(), entry);
    }
}

pub fn get_system_time_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
