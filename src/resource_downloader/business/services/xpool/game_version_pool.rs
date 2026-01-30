use crate::resource_downloader::business::xcache::{
    AnyCacheData, CacheContext, CacheType, CoreCacheManager, FetchFn,
};
use crate::resource_downloader::domain::GameVersion;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct GameVersionPool {
    cache: Arc<CoreCacheManager>,
}

impl GameVersionPool {
    pub fn new(cache: Arc<CoreCacheManager>) -> Self {
        Self { cache }
    }

    /// Fetches the list of game loaders for a specific resource type.
    pub fn get_versions(&self) -> anyhow::Result<Option<Vec<GameVersion>>> {
        let (ctx, fetcher) = self.prepare_request();
        self.cache
            .get::<Vec<GameVersion>>(CacheType::GameVersions, ctx, fetcher)
    }

    /// Fetches the list of game loaders for a specific resource type. Blocks until the request is fulfilled.
    pub async fn get_versions_blocking(&self) -> anyhow::Result<Option<Vec<GameVersion>>> {
        let (ctx, fetcher) = self.prepare_request();
        self.cache
            .get_blocking::<Vec<GameVersion>>(
                CacheType::GameVersions,
                ctx,
                fetcher,
                Duration::from_secs(5),
            )
            .await
    }

    fn prepare_request(&self) -> (CacheContext, FetchFn) {
        let ctx = CacheContext {
            id: None,
            resource_type: None,
            version: None,
            loader: None,
        };

        let fetcher: FetchFn = Box::new(move |p_ctx| {
            Box::pin(async move {
                let data = p_ctx.provider.fetch_release_game_versions(&p_ctx).await?;
                Ok(Arc::new(data) as AnyCacheData)
            })
        });

        (ctx, fetcher)
    }
}
