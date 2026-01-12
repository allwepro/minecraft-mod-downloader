use crate::domain::ModInfo;
use crate::infra::ApiService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ModService {
    api_service: Arc<ApiService>,
    pool: Arc<Mutex<ModInfoPool>>,
}

impl ModService {
    pub fn new(api_service: Arc<ApiService>) -> Self {
        Self {
            api_service,
            pool: Arc::new(Mutex::new(ModInfoPool::new(500, 1))),
        }
    }

    pub async fn get_mod_by_id(
        &self,
        id: &str,
        version: &str,
        loader: &str,
    ) -> Result<Arc<ModInfo>> {
        self.get_mod_internal(id, version, loader, |pool| pool.get(id))
            .await
    }

    pub async fn get_mod_by_slug(
        &self,
        slug: &str,
        version: &str,
        loader: &str,
    ) -> Result<Arc<ModInfo>> {
        self.get_mod_internal(slug, version, loader, |pool| pool.get_by_slug(slug))
            .await
    }

    async fn get_mod_internal<F>(
        &self,
        identifier: &str,
        version: &str,
        loader: &str,
        cache_check: F,
    ) -> Result<Arc<ModInfo>>
    where
        F: FnOnce(&ModInfoPool) -> Option<Arc<ModInfo>>,
    {
        if let Some(info) = {
            let pool = self.pool.lock().await;
            cache_check(&pool)
        } {
            return Ok(info);
        }

        let _permit = self.api_service.limiter.acquire(1).await;

        let details = self
            .api_service
            .provider
            .fetch_mod_details(identifier, version, loader)
            .await?;

        let mut pool = self.pool.lock().await;
        Ok(pool.insert(details))
    }

    pub fn get_cached_mod_blocking(&self, mod_id: &str) -> Option<Arc<ModInfo>> {
        self.pool.blocking_lock().get(mod_id)
    }

    pub fn contains_valid_blocking(&self, mod_id: &str) -> bool {
        self.pool.blocking_lock().contains_valid(mod_id)
    }

    pub fn clear_cache(&self) {
        self.pool.blocking_lock().clear();
    }
    pub fn clear_cache_for(&self, mod_id: &str) {
        let mut pool = self.pool.blocking_lock();
        if let Some(cached) = pool.cache.remove(mod_id) {
            pool.slug_to_id.remove(&cached.info.slug);
        }
    }

    pub async fn cache_search_results(&self, results: Vec<ModInfo>) -> Vec<Arc<ModInfo>> {
        let mut pool = self.pool.lock().await;
        results
            .into_iter()
            .map(|mod_info| pool.insert(mod_info))
            .collect()
    }
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
