use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub mod mod_src;

pub use mod_src::ModProvider;

pub mod mod_service;

pub use mod_service::ModService;

pub mod launcher;

pub use launcher::{JavaInstallation, LaunchConfig, LaunchProfile, LaunchResult, MinecraftInstallation};

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
        mod_info: Arc<ModInfo>,
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
    SearchResults(Vec<Arc<ModInfo>>),
    ModDetails(Arc<ModInfo>),
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
        successful: Vec<Arc<ModInfo>>,
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
struct CachedModInfo {
    pub info: Arc<ModInfo>,
    pub cached_at: DateTime<Utc>,
}

impl CachedModInfo {
    pub fn new(info: Arc<ModInfo>) -> Self {
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

#[derive(Clone)]
pub struct ModInfoPool {
    cache: HashMap<String, CachedModInfo>,
    slug_to_id: HashMap<String, String>,
    max_size: usize,
    max_age_hours: i64,
}

impl ModInfoPool {
    pub fn new(max_size: usize, max_age_hours: i64) -> Self {
        Self {
            cache: HashMap::new(),
            slug_to_id: HashMap::new(),
            max_size,
            max_age_hours,
        }
    }

    pub fn get(&self, mod_id: &str) -> Option<Arc<ModInfo>> {
        self.cache.get(mod_id).and_then(|cached| {
            if cached.is_expired(self.max_age_hours) {
                None
            } else {
                Some(cached.info.clone())
            }
        })
    }

    pub fn get_by_slug(&self, slug: &str) -> Option<Arc<ModInfo>> {
        self.slug_to_id.get(slug).and_then(|id| self.get(id))
    }

    pub fn insert(&mut self, info: ModInfo) -> Arc<ModInfo> {
        let id = info.id.clone();
        let slug = info.slug.clone();
        let arc_info = Arc::new(info);

        if !slug.is_empty() {
            self.slug_to_id.insert(slug, id.clone());
        }

        if let Some(existing) = self.cache.get(&id) {
            if !existing.is_expired(self.max_age_hours) {
                return existing.info.clone();
            }
        }

        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }

        self.cache
            .insert(id.clone(), CachedModInfo::new(arc_info.clone()));
        arc_info
    }

    pub fn contains_valid(&self, mod_id: &str) -> bool {
        self.cache
            .get(mod_id)
            .map(|c| !c.is_expired(self.max_age_hours))
            .unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.slug_to_id.clear();
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .cache
            .iter()
            .min_by_key(|(_, v)| v.cached_at)
            .map(|(k, _)| k.clone())
        {
            if let Some(cached) = self.cache.remove(&oldest_key) {
                self.slug_to_id.remove(&cached.info.slug);
            }
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
