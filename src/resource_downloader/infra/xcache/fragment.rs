use crate::resource_downloader::business::xcache::{AnyCacheData, CacheType};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ProjectLnk, RTProjectData, RTProjectVersion,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type SerializeFn = fn(AnyCacheData) -> anyhow::Result<Vec<u8>>;
pub type DeserializeFn = fn(&[u8]) -> anyhow::Result<AnyCacheData>;

pub struct FragmentEncoder {
    pub serialize: SerializeFn,
    pub deserialize: DeserializeFn,
}

impl FragmentEncoder {
    pub fn json<T>() -> Self
    where
        T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    {
        Self {
            serialize: |data| {
                let concrete = data
                    .downcast_ref::<T>()
                    .ok_or_else(|| anyhow::anyhow!("Downcast failed"))?;
                Ok(serde_json::to_vec(concrete)?)
            },
            deserialize: |bytes| {
                let data: T = serde_json::from_slice(bytes)?;
                Ok(Arc::new(data) as AnyCacheData)
            },
        }
    }

    pub fn binary() -> Self {
        Self {
            serialize: |data| {
                let bytes = data
                    .downcast_ref::<Vec<u8>>()
                    .ok_or_else(|| anyhow::anyhow!("Downcast failed"))?;
                Ok(bytes.clone())
            },
            deserialize: |bytes| Ok(Arc::new(bytes.to_vec()) as AnyCacheData),
        }
    }
}

impl CacheType {
    pub fn encoder(&self) -> FragmentEncoder {
        match self {
            Self::GameLoaders => FragmentEncoder::json::<Vec<GameLoader>>(),
            Self::GameVersions => FragmentEncoder::json::<Vec<GameVersion>>(),
            Self::Search => FragmentEncoder::json::<Vec<ProjectLnk>>(),
            Self::ProjectSlug => FragmentEncoder::json::<String>(),
            Self::ProjectMetadata => FragmentEncoder::json::<RTProjectData>(),
            Self::ProjectVersions => FragmentEncoder::json::<Vec<RTProjectVersion>>(),
            Self::ProjectIcons => FragmentEncoder::binary(),
        }
    }
}
