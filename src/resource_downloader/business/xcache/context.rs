use crate::resource_downloader::business::xcache::common::CacheType;
use crate::resource_downloader::domain::{GameLoader, GameVersion, ResourceType};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CacheContext {
    pub id: Option<String>,
    pub resource_type: Option<ResourceType>,
    pub version: Option<GameVersion>,
    pub loader: Option<GameLoader>,
}

impl CacheContext {
    pub fn hashed_key(&self, ty: CacheType) -> String {
        let mut hasher = DefaultHasher::new();
        ty.hash(&mut hasher);
        self.id.hash(&mut hasher);
        self.resource_type.hash(&mut hasher);
        self.version.hash(&mut hasher);
        self.loader.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
