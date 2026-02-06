mod game;
mod lnk_types;
mod project;
mod project_list;
mod project_operations;

pub use game::*;
pub use lnk_types::*;
pub use project::*;
pub use project_list::*;
pub use project_operations::*;
use serde::{Deserialize, Serialize};
// ---------------- Runtime Project ----------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RTProjectDependency {
    pub project: ProjectLnk,
    pub dependency_type: ProjectDependencyType,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RTProjectVersion {
    pub version_id: String,
    pub version_name: String,
    pub artifact_id: String,
    ///sha1
    pub artifact_hash: String,
    pub channel: String,
    pub depended_on: Vec<RTProjectDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RTProjectData {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub icon_url: String,
    pub download_count: u32,
    pub supported_versions: Vec<GameVersion>,
    pub supported_loaders: Vec<GameLoader>,
}

// ---------------- Resource Type ----------------
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
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

pub const RESOURCE_TYPES: [ResourceType; 5] = [
    ResourceType::Mod,
    ResourceType::ResourcePack,
    ResourceType::Shader,
    ResourceType::Datapack,
    ResourceType::Plugin,
];

#[allow(dead_code)]
impl ResourceType {
    pub fn from_str(s: String) -> Option<Self> {
        match s.as_str() {
            "mod" => Some(Self::Mod),
            "resourcepack" | "texturepack" => Some(Self::ResourcePack),
            "shader" | "shaderpack" => Some(Self::Shader),
            "datapack" => Some(Self::Datapack),
            "plugin" => Some(Self::Plugin),
            _ => None,
        }
    }

    pub fn id(&self) -> String {
        match self {
            Self::Mod => "mod",
            Self::ResourcePack => "resourcepack",
            Self::Shader => "shader",
            Self::Datapack => "datapack",
            Self::Plugin => "plugin",
        }
        .parse()
        .unwrap()
    }

    pub fn emoji(&self) -> String {
        match self {
            Self::Mod => "⚒",
            Self::ResourcePack => "🖼",
            Self::Shader => "✨",
            Self::Datapack => "📦",
            Self::Plugin => "🔌",
        }
        .parse()
        .unwrap()
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Mod => "Mod",
            Self::ResourcePack => "Resource Pack",
            Self::Shader => "Shader",
            Self::Datapack => "Data Pack",
            Self::Plugin => "Plugin",
        }
        .parse()
        .unwrap()
    }

    pub fn game_folder(&self) -> String {
        match self {
            Self::Mod => "mods",
            Self::ResourcePack => "resourcepacks",
            Self::Shader => "shaderpacks",
            _ => ".",
        }
        .to_string()
    }

    pub fn server_folder(&self) -> Option<String> {
        match self {
            Self::Plugin => Some("plugins"),
            _ => None,
        }
        .map(|s| s.to_string())
    }

    pub fn world_folder(&self) -> Option<String> {
        match self {
            Self::Datapack => Some("datapacks"),
            _ => None,
        }
        .map(|s| s.to_string())
    }

    pub fn file_extension(&self) -> String {
        match self {
            Self::Mod => "jar",
            Self::ResourcePack => "zip",
            Self::Shader => "zip",
            Self::Datapack => "zip",
            Self::Plugin => "jar",
        }
        .parse()
        .unwrap()
    }
}

/*
DO NOT IMPLEMENT THAT - as it would throw if an invalid resource type is passed. ResourceType::from_str() should be used instead.
impl From<String> for ResourceType {
    fn from(s: String) -> Self {
        Self::from_str(s.clone()).unwrap_or_else(|| panic!("Invalid resource type: {}", s))
    }
}*/

// ---------------- Config ----------------
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_list_name")]
    pub default_list_name: String,
    pub last_open_list_id: Option<ListLnk>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_list_name: default_list_name(),
            last_open_list_id: None,
        }
    }
}

fn default_list_name() -> String {
    "New List".to_string()
}
