use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minecraft version manifest (the version.json file)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub assets: String,
    #[serde(rename = "type")]
    pub version_type: String,
    #[serde(rename = "inheritsFrom")]
    #[serde(default)]
    pub inherits_from: Option<String>,
}

/// Modern argument format (1.13+)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgumentValue>,
    #[serde(default)]
    pub jvm: Vec<ArgumentValue>,
}

/// Argument can be a string or a conditional object
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    String(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValueInner,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentValueInner {
    String(String),
    Array(Vec<String>),
}

/// Library dependency
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// Rule for conditional inclusion
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

/// Asset index reference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

impl VersionManifest {
    /// Load version manifest from JSON file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: VersionManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Check if library should be included based on rules
    pub fn should_include_library(&self, library: &Library) -> bool {
        if library.rules.is_empty() {
            return true;
        }

        let os_name = Self::get_os_name();
        let mut allowed = false;

        for rule in &library.rules {
            let matches = if let Some(ref os) = rule.os {
                os.name.as_ref().map(|n| n == os_name).unwrap_or(true)
            } else {
                true
            };

            if matches {
                allowed = rule.action == "allow";
            }
        }

        allowed
    }

    /// Get current OS name in Minecraft format
    fn get_os_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        }
    }

    /// Get platform-specific classifier (for natives)
    pub fn get_natives_classifier() -> &'static str {
        if cfg!(target_os = "windows") {
            "natives-windows"
        } else if cfg!(target_os = "macos") {
            "natives-macos"
        } else {
            "natives-linux"
        }
    }
}
