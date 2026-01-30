use crate::resource_downloader::business::xcache::{
    AnyCacheData, CacheContext, CacheType, CoreCacheManager, FetchFn,
};
use crate::resource_downloader::domain::{GameLoader, ResourceType};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct GameLoaderPool {
    cache: Arc<CoreCacheManager>,
}

impl GameLoaderPool {
    pub fn new(cache: Arc<CoreCacheManager>) -> Self {
        Self { cache }
    }

    /// Fetches the list of game loaders for a specific resource type.
    pub fn get_loaders(
        &self,
        resource_type: ResourceType,
    ) -> anyhow::Result<Option<Vec<GameLoader>>> {
        let (ctx, fetcher) = self.prepare_request(resource_type);
        self.cache
            .get::<Vec<GameLoader>>(CacheType::GameLoaders, ctx, fetcher)
    }

    /// Fetches the list of game loaders for a specific resource type. Blocks until the request is fulfilled.
    pub async fn get_loaders_blocking(
        &self,
        resource_type: ResourceType,
    ) -> anyhow::Result<Option<Vec<GameLoader>>> {
        let (ctx, fetcher) = self.prepare_request(resource_type);
        self.cache
            .get_blocking::<Vec<GameLoader>>(
                CacheType::GameLoaders,
                ctx,
                fetcher,
                Duration::from_secs(5),
            )
            .await
    }

    /// Fetches the game loader for a specific id for a specific resource type. Blocks until the request is fulfilled.
    pub async fn get_loader_by_id_blocking(
        &self,
        id: String,
        resource_type: &ResourceType,
    ) -> anyhow::Result<Option<GameLoader>> {
        let loaders = self.get_loaders_blocking(*resource_type).await?;
        Ok(loaders.and_then(|g| g.into_iter().find(|l| l.id == id)))
    }

    pub fn warm_loader(&self, resource_type: ResourceType, data: Vec<GameLoader>) {
        let (ctx, _) = self.prepare_request(resource_type);
        self.cache
            .warm(CacheType::GameLoaders, ctx, Arc::new(data) as AnyCacheData)
    }

    fn prepare_request(&self, resource_type: ResourceType) -> (CacheContext, FetchFn) {
        let ctx = CacheContext {
            id: None,
            resource_type: Some(resource_type),
            version: None,
            loader: None,
        };

        let fetcher: FetchFn = Box::new(move |p_ctx| {
            Box::pin(async move {
                let data = p_ctx
                    .provider
                    .fetch_game_loaders_for_resource_type(&p_ctx, resource_type)
                    .await?;
                Ok(Arc::new(data) as AnyCacheData)
            })
        });

        (ctx, fetcher)
    }
}
