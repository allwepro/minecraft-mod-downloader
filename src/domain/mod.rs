use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod mod_source;

pub use mod_source::ModProvider;

pub mod mod_service;

use crate::infra::DownloadMetadata;
pub use mod_service::ModService;

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
        .trim_matches(|c| c == '_' || c == '-')
        .to_string()
}

pub fn generate_mod_filename(mod_info: &ModInfo) -> String {
    let sanitized_name = sanitize_filename(&mod_info.name);
    let extension = mod_info.project_type.fileext();
    format!("{sanitized_name}.{extension}")
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ProjectType {
    #[default]
    #[serde(rename = "mod")]
    Mod,
    #[serde(rename = "resourcepack")]
    ResourcePack,
    #[serde(rename = "shader")]
    Shader,
    #[serde(rename = "datapack")]
    Datapack,
    #[serde(rename = "plugin")]
    Plugin,
}

impl ProjectType {
    pub fn id(&self) -> &str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::ResourcePack => "resourcepack",
            ProjectType::Shader => "shader",
            ProjectType::Datapack => "datapack",
            ProjectType::Plugin => "plugin",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ProjectType::Mod => "Mod",
            ProjectType::ResourcePack => "Resource Pack",
            ProjectType::Shader => "Shader",
            ProjectType::Datapack => "Data Pack",
            ProjectType::Plugin => "Plugin",
        }
    }

    pub fn fileext(&self) -> &str {
        match self {
            ProjectType::Mod => "jar",
            ProjectType::ResourcePack => "zip",
            ProjectType::Shader => "zip",
            ProjectType::Datapack => "zip",
            ProjectType::Plugin => "jar",
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            ProjectType::Mod => "⚒",
            ProjectType::ResourcePack => "🖼",
            ProjectType::Shader => "✨",
            ProjectType::Datapack => "📦",
            ProjectType::Plugin => "🔌",
        }
    }
}

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
    #[serde(default)]
    pub project_type: ProjectType,
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
    #[serde(default)]
    pub compatibility_override: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModList {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub mods: Vec<ModEntry>,
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_modloader", deserialize_with = "deserialize_loader")]
    pub loader: ModLoader,
    #[serde(default)]
    pub download_dir: String,
    #[serde(default)]
    pub content_type: ProjectType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub current_list_id: Option<String>,
    #[serde(default = "default_list_name")]
    pub default_list_name: String,
}

fn default_list_name() -> String {
    "New List".to_string()
}

fn default_modloader() -> ModLoader {
    ModLoader {
        id: String::new(),
        name: String::new(),
    }
}

fn deserialize_loader<'de, D>(deserializer: D) -> Result<ModLoader, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let value = serde_json::Value::deserialize(deserializer).map_err(D::Error::custom)?;

    if value.is_string()
        && let Some(s) = value.as_str()
    {
        return Ok(ModLoader {
            id: s.to_string(),
            name: s.to_string(),
        });
    }

    let ml: ModLoader = serde_json::from_value(value).map_err(D::Error::custom)?;
    Ok(ml)
}

pub enum Event {
    InitialDataLoaded {
        mod_lists: Vec<ModList>,
        current_list_id: Option<String>,
        minecraft_versions: Vec<MinecraftVersion>,
        mod_loaders: Vec<ModLoader>,
        default_list_name: String,
    },
    LoadersForTypeLoaded {
        project_type: ProjectType,
        loaders: Vec<ModLoader>,
    },
    SearchResults(Vec<Arc<ModInfo>>),
    ModDetails {
        info: Arc<ModInfo>,
        version: String,
        loader: String,
    },
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
    MetadataLoaded {
        download_dir: String,
        metadata: DownloadMetadata,
    },
    ModrinthCollectionLoaded {
        name: String,
        recommended_version: String,
        recommended_loader: String,
        projects: Vec<(String, String)>, // (project_id, project_name)
    },
    ModrinthCollectionFailed {
        error: String,
    },
}
