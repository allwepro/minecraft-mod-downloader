use crate::domain::ModInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

const CACHE_DURATION_HOURS: u64 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedProject {
    mod_info: ModInfo,
    cached_at: u64,
}

impl CachedProject {
    fn new(mod_info: ModInfo) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        Self {
            mod_info,
            cached_at,
        }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        let age_seconds = now.saturating_sub(self.cached_at);
        age_seconds >= CACHE_DURATION_HOURS * 60 * 60
    }
}

pub struct ProjectCache {
    cache_dir: PathBuf,
    memory_cache: RwLock<HashMap<String, CachedProject>>,
}

impl ProjectCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir: cache_dir.join("data"),
            memory_cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(&self, mod_id: &str, version: &str, loader: &str) -> Option<ModInfo> {
        let key = Self::make_key(mod_id, version, loader);

        {
            let memory_cache = self.memory_cache.read().await;
            if let Some(cached) = memory_cache.get(&key)
                && !cached.is_expired()
            {
                return Some(cached.mod_info.clone());
            }
        }

        let cache_path = self.cache_path(&key);
        if cache_path.exists()
            && let Ok(content) = tokio::fs::read_to_string(&cache_path).await
            && let Ok(cached) = serde_json::from_str::<CachedProject>(&content)
        {
            if !cached.is_expired() {
                let mut memory_cache = self.memory_cache.write().await;
                memory_cache.insert(key.clone(), cached.clone());
                return Some(cached.mod_info);
            } else {
                let _ = tokio::fs::remove_file(&cache_path).await;
            }
        }

        None
    }

    pub async fn set(&self, mod_id: &str, version: &str, loader: &str, mod_info: ModInfo) {
        let key = Self::make_key(mod_id, version, loader);
        let cached = CachedProject::new(mod_info);

        {
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.insert(key.clone(), cached.clone());
        }

        let cache_path = self.cache_path(&key);
        if let Some(parent) = cache_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        if let Ok(json) = serde_json::to_string(&cached) {
            let _ = tokio::fs::write(&cache_path, json).await;
        }
    }

    pub async fn clear_expired(&self) {
        {
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.retain(|_, cached| !cached.is_expired());
        }

        if let Ok(mut entries) = tokio::fs::read_dir(&self.cache_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file()
                    && let Ok(content) = tokio::fs::read_to_string(&path).await
                    && let Ok(cached) = serde_json::from_str::<CachedProject>(&content)
                    && cached.is_expired()
                {
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
        }
    }

    fn make_key(mod_id: &str, version: &str, loader: &str) -> String {
        format!("{}_{}_{}", mod_id, version, loader)
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", key))
    }
}
