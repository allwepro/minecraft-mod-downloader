use crate::domain::ModInfo;
use crate::infra::{ApiService, ProjectCache};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ModService {
    pub(crate) api_service: Arc<ApiService>,
    pool: Arc<Mutex<ModInfoPool>>,
    disk_cache: Arc<ProjectCache>,
}

impl ModService {
    pub fn new(api_service: Arc<ApiService>, cache_dir: std::path::PathBuf) -> Self {
        Self {
            api_service,
            pool: Arc::new(Mutex::new(ModInfoPool::new(500, 1))),
            disk_cache: Arc::new(ProjectCache::new(cache_dir)),
        }
    }

    pub fn get_disk_cache(&self) -> Arc<ProjectCache> {
        self.disk_cache.clone()
    }

    pub async fn get_mod_by_id(
        &self,
        id: &str,
        version: &str,
        loader: &str,
    ) -> Result<Arc<ModInfo>> {
        self.get_mod_internal(id, version, loader, |pool, v, l| pool.get(id, v, l))
            .await
    }

    pub async fn get_mod_by_slug(
        &self,
        slug: &str,
        version: &str,
        loader: &str,
    ) -> Result<Arc<ModInfo>> {
        self.get_mod_internal(slug, version, loader, |pool, v, l| {
            pool.get_by_slug(slug, v, l)
        })
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
        F: FnOnce(&ModInfoPool, &str, &str) -> Option<Arc<ModInfo>>,
    {
        if let Some(info) = {
            let pool = self.pool.lock().await;
            cache_check(&pool, version, loader)
        } {
            if !info.version.is_empty() {
                log::debug!("Returning cached complete info for {}", identifier);
                return Ok(info);
            }
            log::debug!(
                "Cached info for {} has empty version, fetching fresh",
                identifier
            );
        }

        if let Some(cached_info) = self.disk_cache.get(identifier, version, loader).await {
            log::debug!("Returning disk-cached info for {}", identifier);
            let mut pool = self.pool.lock().await;
            return Ok(pool.insert(cached_info, version.to_string(), loader.to_string()));
        }

        let _permit = self.api_service.limiter.acquire(1).await;

        log::debug!(
            "Fetching mod details for {} (version={} loader={})",
            identifier,
            version,
            loader
        );

        let details = self
            .api_service
            .provider
            .fetch_mod_details(identifier, version, loader)
            .await?;

        self.disk_cache
            .set(identifier, version, loader, details.clone())
            .await;

        let mut pool = self.pool.lock().await;
        Ok(pool.insert(details, version.to_string(), loader.to_string()))
    }

    pub async fn cache_search_results(
        &self,
        results: Vec<ModInfo>,
        version: String,
        loader: String,
    ) -> Vec<Arc<ModInfo>> {
        let mut pool = self.pool.lock().await;
        results
            .into_iter()
            .map(|mod_info| pool.insert(mod_info, version.clone(), loader.clone()))
            .collect()
    }
}

#[derive(Clone, Debug)]
struct CachedModInfo {
    pub info: Arc<ModInfo>,
    pub cached_at: DateTime<Utc>,
    pub version: String,
    pub loader: String,
}

impl CachedModInfo {
    pub fn new(info: Arc<ModInfo>, version: String, loader: String) -> Self {
        Self {
            info,
            cached_at: Utc::now(),
            version,
            loader,
        }
    }

    pub fn is_expired(&self, max_age_hours: i64) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.cached_at);
        age.num_hours() >= max_age_hours
    }

    pub fn matches_context(&self, version: &str, loader: &str) -> bool {
        self.version == version && self.loader == loader
    }
}

#[derive(Clone)]
pub struct ModInfoPool {
    cache: HashMap<String, CachedModInfo>,
    base_info_cache: HashMap<String, Arc<ModInfo>>,
    slug_to_id: HashMap<String, String>,
    max_size: usize,
    max_age_hours: i64,
}

impl ModInfoPool {
    pub fn new(max_size: usize, max_age_hours: i64) -> Self {
        Self {
            cache: HashMap::new(),
            base_info_cache: HashMap::new(),
            slug_to_id: HashMap::new(),
            max_size,
            max_age_hours,
        }
    }

    pub fn get(&self, mod_id: &str, version: &str, loader: &str) -> Option<Arc<ModInfo>> {
        self.cache.get(mod_id).and_then(|cached| {
            if cached.is_expired(self.max_age_hours) || !cached.matches_context(version, loader) {
                None
            } else {
                Some(cached.info.clone())
            }
        })
    }

    pub fn get_by_slug(&self, slug: &str, version: &str, loader: &str) -> Option<Arc<ModInfo>> {
        self.slug_to_id
            .get(slug)
            .and_then(|id| self.get(id, version, loader))
    }

    pub fn insert(&mut self, info: ModInfo, version: String, loader: String) -> Arc<ModInfo> {
        let id = info.id.clone();
        let slug = info.slug.clone();
        let arc_info = Arc::new(info);

        if !slug.is_empty() {
            self.slug_to_id.insert(slug, id.clone());
        }

        self.base_info_cache.insert(id.clone(), arc_info.clone());

        if let Some(existing) = self.cache.get(&id)
            && !existing.is_expired(self.max_age_hours)
            && existing.matches_context(&version, &loader)
        {
            if !existing.info.version.is_empty() {
                log::debug!("Keeping existing complete cache entry for {}", id);
                return existing.info.clone();
            }
            log::debug!("Replacing incomplete cache entry for {} with new data", id);
        }

        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }

        log::debug!(
            "Caching mod {}: version='{}', download_url present={}",
            id,
            arc_info.version,
            !arc_info.download_url.is_empty()
        );

        self.cache.insert(
            id.clone(),
            CachedModInfo::new(arc_info.clone(), version, loader),
        );
        arc_info
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .cache
            .iter()
            .min_by_key(|(_, v)| v.cached_at)
            .map(|(k, _)| k.clone())
            && let Some(cached) = self.cache.remove(&oldest_key)
        {
            self.slug_to_id.remove(&cached.info.slug);
            // Don't remove from base_info_cache as it's version-independent
        }
    }
}
