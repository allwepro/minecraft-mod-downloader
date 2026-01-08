use crate::domain::{ConnectionLimiter, ModInfo, ModInfoPool, ModProvider};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ModService {
    provider: Arc<dyn ModProvider>,
    limiter: Arc<ConnectionLimiter>,
    pool: Arc<Mutex<ModInfoPool>>,
}

impl ModService {
    pub fn new(
        provider: Arc<dyn ModProvider>,
        limiter: Arc<ConnectionLimiter>,
        pool: Arc<Mutex<ModInfoPool>>,
    ) -> Self {
        Self {
            provider,
            limiter,
            pool,
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

        let _permit = self.limiter.acquire(1).await;

        let details = self
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

    pub async fn cache_search_results(&self, results: Vec<ModInfo>) -> Vec<Arc<ModInfo>> {
        let mut pool = self.pool.lock().await;
        results
            .into_iter()
            .map(|mod_info| pool.insert(mod_info))
            .collect()
    }
}
