use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a Minecraft launch profile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaunchProfile {
    pub minecraft_version: String,
    pub mod_loader: String,
    pub mod_loader_version: Option<String>,
    pub java_path: PathBuf,
    pub game_directory: PathBuf,
    pub mod_list_id: Option<String>,
}

/// Launch configuration for starting Minecraft
#[derive(Clone, Debug)]
pub struct LaunchConfig {
    pub profile: LaunchProfile,
    pub username: String,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            profile: LaunchProfile {
                minecraft_version: "1.20.1".to_string(),
                mod_loader: "fabric".to_string(),
                mod_loader_version: None,
                java_path: PathBuf::new(),
                game_directory: PathBuf::new(),
                mod_list_id: None,
            },
            username: "Player".to_string(),
            max_memory_mb: 4096,
            min_memory_mb: 1024,
        }
    }
}

/// Result of a launch attempt
#[derive(Clone, Debug)]
pub enum LaunchResult {
    Success { pid: u32 },
    Failed { error: String },
}

/// Detected Java installation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
    pub is_valid: bool,
}

/// Detected Minecraft installation
#[derive(Clone, Debug)]
pub struct MinecraftInstallation {
    pub root_dir: PathBuf,
    pub versions_dir: PathBuf,
    pub mods_dir: PathBuf,
    pub available_versions: Vec<String>,
}
