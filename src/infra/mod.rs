// Origin/main infrastructure
mod api_service;
mod config_manager;
mod download_metadata;
mod icon_service;
mod icon_worker;
mod legacy_list;
mod project_cache;

// Launcher infrastructure
mod java_detector;
mod minecraft_detector;
mod mod_copier;
mod natives_extractor;

// Origin/main exports
pub use api_service::ApiService;
pub use config_manager::ConfigManager;
pub use download_metadata::{
    DownloadMetadata, read_download_metadata, remove_metadata_entry, update_metadata_entry,
    write_download_metadata,
};
pub use icon_service::IconService;
pub use icon_worker::IconWorker;
pub use legacy_list::LegacyListService;
pub use project_cache::ProjectCache;

// Launcher exports
pub use java_detector::JavaDetector;
pub use minecraft_detector::MinecraftDetector;
pub use mod_copier::ModCopier;
pub use natives_extractor::NativesExtractor;
