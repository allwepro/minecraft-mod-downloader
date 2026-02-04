use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Minecraft version manifest (the version.json file)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub id: String,
    #[serde(rename = "mainClass")]
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub jar: Option<String>,
    #[serde(default)]
    pub downloads: Option<VersionDownloads>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub version_type: Option<String>,
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
    #[serde(default)]
    pub url: Option<String>,
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

/// Version-level downloads (client/server jars)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionDownloads {
    #[serde(default)]
    pub client: Option<VersionDownloadItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionDownloadItem {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// Fully-resolved manifest after applying inheritance
#[derive(Debug, Clone)]
pub struct ResolvedManifest {
    pub id: String,
    pub main_class: String,
    pub client_jar_id: String,
    pub arguments: Option<Arguments>,
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndex,
    pub assets: String,
    pub version_type: String,
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
        Self::should_include_library_for_current_os(library)
    }

    /// Check if library should be included for the current OS (static helper)
    pub fn should_include_library_for_current_os(library: &Library) -> bool {
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

    /// Resolve inheritance chain and produce a fully-resolved manifest
    pub fn resolve_from_file(path: &Path) -> anyhow::Result<ResolvedManifest> {
        let versions_dir = path
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| anyhow::anyhow!("Invalid version manifest path"))?;

        let manifest = VersionManifest::from_file(path)?;
        let client_jar_id = manifest
            .jar
            .clone()
            .or_else(|| manifest.inherits_from.clone())
            .unwrap_or_else(|| manifest.id.clone());

        let merged = manifest.resolve_inheritance(versions_dir)?;
        merged.into_resolved_with_client_jar(client_jar_id)
    }

    fn resolve_inheritance(self, versions_dir: &Path) -> anyhow::Result<VersionManifest> {
        if let Some(parent_id) = &self.inherits_from {
            let parent_path = versions_dir
                .join(parent_id)
                .join(format!("{}.json", parent_id));
            let parent = VersionManifest::from_file(&parent_path)?
                .resolve_inheritance(versions_dir)?;
            Ok(VersionManifest::merge(parent, self))
        } else {
            Ok(self)
        }
    }

    fn merge(parent: VersionManifest, child: VersionManifest) -> VersionManifest {
        let libraries = merge_libraries(parent.libraries, child.libraries);
        let arguments = merge_arguments(parent.arguments, child.arguments);

        VersionManifest {
            id: child.id,
            main_class: child.main_class.or(parent.main_class),
            jar: child.jar.or(parent.jar),
            downloads: child.downloads.or(parent.downloads),
            arguments,
            minecraft_arguments: child.minecraft_arguments.or(parent.minecraft_arguments),
            libraries,
            asset_index: child.asset_index.or(parent.asset_index),
            assets: child.assets.or(parent.assets),
            version_type: child.version_type.or(parent.version_type),
            inherits_from: None,
        }
    }

    fn into_resolved_with_client_jar(self, client_jar_id: String) -> anyhow::Result<ResolvedManifest> {
        Ok(ResolvedManifest {
            id: self.id,
            main_class: self
                .main_class
                .ok_or_else(|| anyhow::anyhow!("Manifest missing mainClass"))?,
            client_jar_id,
            arguments: self.arguments,
            minecraft_arguments: self.minecraft_arguments,
            libraries: self.libraries,
            asset_index: self
                .asset_index
                .ok_or_else(|| anyhow::anyhow!("Manifest missing assetIndex"))?,
            assets: self
                .assets
                .ok_or_else(|| anyhow::anyhow!("Manifest missing assets"))?,
            version_type: self
                .version_type
                .ok_or_else(|| anyhow::anyhow!("Manifest missing type"))?,
        })
    }
}

impl ResolvedManifest {
    /// Check if library should be included based on rules
    pub fn should_include_library(&self, library: &Library) -> bool {
        VersionManifest::should_include_library_for_current_os(library)
    }
}

fn merge_arguments(
    parent: Option<Arguments>,
    child: Option<Arguments>,
) -> Option<Arguments> {
    match (parent, child) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(c)) => Some(c),
        (Some(mut p), Some(c)) => {
            p.game.extend(c.game);
            p.jvm.extend(c.jvm);
            Some(p)
        }
    }
}

fn merge_libraries(parent: Vec<Library>, child: Vec<Library>) -> Vec<Library> {
    let mut libraries = parent;
    let mut index = HashMap::new();

    for (i, lib) in libraries.iter().enumerate() {
        index.insert(lib.name.clone(), i);
    }

    for lib in child {
        if let Some(i) = index.get(&lib.name).copied() {
            libraries[i] = lib;
        } else {
            index.insert(lib.name.clone(), libraries.len());
            libraries.push(lib);
        }
    }

    libraries
}
