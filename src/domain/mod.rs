use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod mod_source;

pub use mod_source::ModProvider;

pub mod mod_service;

pub use mod_service::ModService;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModInfo {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub icon_url: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub download_count: u32,
    pub download_url: String,
    pub supported_versions: Vec<String>,
    pub supported_loaders: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModLoader {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModEntry {
    pub mod_id: String,
    pub mod_name: String,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModList {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub mods: Vec<ModEntry>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub loader: String,
    #[serde(default)]
    pub download_dir: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub selected_version: String,
    pub selected_loader: String,
    pub current_list_id: Option<String>,
    pub download_dir: String,
}

pub enum Command {
    SearchMods {
        query: String,
        version: String,
        loader: String,
    },
    FetchModDetails {
        mod_id: String,
        version: String,
        loader: String,
    },
    DownloadMod {
        mod_info: Arc<ModInfo>,
        download_dir: String,
    },
    LegacyListImport {
        path: std::path::PathBuf,
        version: String,
        loader: String,
    },
    LegacyListExport {
        path: std::path::PathBuf,
        mod_ids: Vec<String>,
        version: String,
        loader: String,
    },
}

pub enum Event {
    SearchResults(Vec<Arc<ModInfo>>),
    ModDetails(Arc<ModInfo>),
    ModDetailsFailed {
        mod_id: String,
    },
    DownloadProgress {
        mod_id: String,
        progress: f32,
    },
    DownloadComplete {
        mod_id: String,
        success: bool,
    },
    LegacyListProgress {
        current: usize,
        total: usize,
        message: String,
    },
    LegacyListComplete {
        suggested_name: String,
        successful: Vec<Arc<ModInfo>>,
        failed: Vec<String>,
        warnings: Vec<String>,
        is_import: bool,
    },
    LegacyListFailed {
        error: String,
        is_import: bool,
    },
}
