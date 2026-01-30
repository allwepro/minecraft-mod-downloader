use crate::resource_downloader::business::xcache::common::CacheType;
use crate::resource_downloader::infra::cache::{
    duration_from_days, duration_from_hours, duration_from_minutes,
};
use std::time::Duration;

pub(crate) struct CacheConfig {
    pub(crate) ttl: Duration,
    pub(crate) sub_dir: &'static str,
    pub(crate) concurrency: usize,
}

impl CacheType {
    pub(crate) fn config(&self) -> CacheConfig {
        match self {
            Self::GameLoaders => CacheConfig {
                ttl: duration_from_days(7),
                sub_dir: "loaders",
                concurrency: 2,
            },
            Self::GameVersions => CacheConfig {
                ttl: duration_from_hours(2),
                sub_dir: "versions",
                concurrency: 2,
            },
            Self::Search => CacheConfig {
                ttl: duration_from_minutes(5),
                sub_dir: "search",
                concurrency: 2,
            },
            Self::ProjectSlug => CacheConfig {
                ttl: duration_from_hours(5),
                sub_dir: "prj_slug",
                concurrency: 4,
            },
            Self::ProjectMetadata => CacheConfig {
                ttl: duration_from_hours(5),
                sub_dir: "prj_metadata",
                concurrency: 4,
            },
            Self::ProjectVersions => CacheConfig {
                ttl: duration_from_hours(5),
                sub_dir: "prj_versions",
                concurrency: 4,
            },
            Self::ProjectIcons => CacheConfig {
                ttl: duration_from_days(14),
                sub_dir: "prj_icons",
                concurrency: 3,
            },
        }
    }
}
