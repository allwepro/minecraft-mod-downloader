use crate::resource_downloader::business::xcache::CacheContext;
use crate::resource_downloader::infra::adapters::ResourceProviderContext;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheType {
    GameLoaders,
    GameVersions,
    Search,
    ProjectSlug,
    ProjectMetadata,
    ProjectVersions,
    ProjectIcons,
}

pub(crate) const CACHE_TYPES: [CacheType; 7] = [
    CacheType::GameLoaders,
    CacheType::GameVersions,
    CacheType::Search,
    CacheType::ProjectSlug,
    CacheType::ProjectMetadata,
    CacheType::ProjectVersions,
    CacheType::ProjectIcons,
];

pub type AnyCacheData = Arc<dyn Any + Send + Sync>;
pub type FetchFn = Box<
    dyn FnOnce(ResourceProviderContext) -> BoxFuture<'static, anyhow::Result<AnyCacheData>> + Send,
>;

pub enum CacheCommand {
    Fetch {
        ty: CacheType,
        ctx: CacheContext,
        fetcher: FetchFn,
    },
    Inject {
        ty: CacheType,
        ctx: CacheContext,
        data: AnyCacheData,
    },
    Discard {
        ty: CacheType,
        ctx: CacheContext,
    },
    Cleanup,
}

pub enum CacheResponse {
    Updated {
        ty: CacheType,
        ctx: CacheContext,
        data: AnyCacheData,
        ts: u64,
    },
    FetchFailed {
        ty: CacheType,
        ctx: CacheContext,
        error: String,
    },
}

pub struct CacheEntry {
    pub data: AnyCacheData,
    pub updated_at: u64,
}
