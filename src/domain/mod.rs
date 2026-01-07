use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub mod mod_src;

pub use mod_src::ModProvider;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModInfo {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub download_count: u32,
    pub download_url: String,
    pub supported_versions: Vec<String>,
    pub supported_loaders: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Idle,
    Queued,
    Downloading,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModLoader {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModEntry {
    pub mod_id: String,
    pub mod_name: String,
    pub added_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModList {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub mods: Vec<ModEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub selected_version: String,
    pub selected_loader: String,
    pub current_list_id: Option<String>,
    pub download_dir: String,
}

pub enum Command {
    SearchMods {
        query: String,
        version: String,
        loader: String,
    },
    FetchModDetails {
        mod_id: String,
        version: String,
        loader: String,
    },
    DownloadMod {
        mod_info: ModInfo,
        download_dir: String,
    },
    LegacyListImport {
        path: std::path::PathBuf,
        version: String,
        loader: String,
    },
    LegacyListExport {
        path: std::path::PathBuf,
        mod_ids: Vec<String>,
        version: String,
        loader: String,
    },
}

pub enum Event {
    SearchResults(Vec<ModInfo>),
    ModDetails(ModInfo),
    ModDetailsFailed {
        mod_id: String,
    },
    DownloadProgress {
        mod_id: String,
        progress: f32,
    },
    DownloadComplete {
        mod_id: String,
        success: bool,
    },
    LegacyListProgress {
        current: usize,
        total: usize,
        message: String,
    },
    LegacyListComplete {
        successful: Vec<ModInfo>,
        failed: Vec<String>,
        warnings: Vec<String>,
        is_import: bool,
    },
    LegacyListFailed {
        error: String,
        is_import: bool,
    },
}

#[derive(Clone, Debug)]
pub struct CachedModInfo {
    pub info: ModInfo,
    pub cached_at: DateTime<Utc>,
}

impl CachedModInfo {
    pub fn new(info: ModInfo) -> Self {
        Self {
            info,
            cached_at: Utc::now(),
        }
    }

    pub fn is_expired(&self, max_age_hours: i64) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.cached_at);
        age.num_hours() >= max_age_hours
    }
}

pub struct ModCache {
    pub cache: HashMap<String, CachedModInfo>,
    max_size: usize,
    max_age_hours: i64,
}

impl ModCache {
    pub fn new(max_size: usize, max_age_hours: i64) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            max_age_hours,
        }
    }

    pub fn get(&self, mod_id: &str) -> Option<ModInfo> {
        self.cache.get(mod_id).and_then(|cached| {
            if cached.is_expired(self.max_age_hours) {
                None
            } else {
                Some(cached.info.clone())
            }
        })
    }

    pub fn insert(&mut self, mod_id: String, info: ModInfo) {
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }
        self.cache.insert(mod_id, CachedModInfo::new(info));
    }

    pub fn contains_valid(&self, mod_id: &str) -> bool {
        self.cache
            .get(mod_id)
            .map(|c| !c.is_expired(self.max_age_hours))
            .unwrap_or(false)
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .cache
            .iter()
            .min_by_key(|(_, v)| v.cached_at)
            .map(|(k, _)| k.clone())
        {
            self.cache.remove(&oldest_key);
        }
    }

    pub fn clear_expired(&mut self) {
        self.cache.retain(|_, v| !v.is_expired(self.max_age_hours));
    }
}

#[derive(Clone)]
pub struct ConnectionLimiter {
    semaphore: Arc<Semaphore>,
}

impl ConnectionLimiter {
    pub fn new(max_connections: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    pub async fn acquire(&self, slots: u32) -> tokio::sync::OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_many_owned(slots)
            .await
            .expect("Semaphore closed")
    }
}
