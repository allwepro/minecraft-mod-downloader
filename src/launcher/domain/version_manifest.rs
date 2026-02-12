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
    pub main_class: String,
    pub client_jar_id: String,
    pub arguments: Option<Arguments>,
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndex,
}

impl VersionManifest {
    /// Load version manifest from JSON file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: VersionManifest = serde_json::from_str(&content)?;
        Ok(manifest)
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
                let name_ok = os.name.as_ref().map(|n| n == os_name).unwrap_or(true);
                let arch_ok = os
                    .arch
                    .as_ref()
                    .map(|arch| Self::matches_arch(arch.as_str()))
                    .unwrap_or(true);
                name_ok && arch_ok
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

    fn matches_arch(arch: &str) -> bool {
        match arch {
            "x86" => cfg!(target_pointer_width = "32"),
            "x86_64" => cfg!(target_arch = "x86_64"),
            "arm64" | "aarch64" => cfg!(target_arch = "aarch64"),
            _ => true,
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

    fn into_resolved_with_client_jar(
        self,
        client_jar_id: String,
    ) -> anyhow::Result<ResolvedManifest> {
        Ok(ResolvedManifest {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "mmd_manifest_{}_{}_{}",
            name,
            std::process::id(),
            nonce
        ));
        std::fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn sample_library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            rules: Vec::new(),
            natives: None,
            downloads: None,
            url: None,
        }
    }

    fn sample_asset(id: &str) -> AssetIndex {
        AssetIndex {
            id: id.to_string(),
            sha1: "deadbeef".to_string(),
            size: 1,
            total_size: 1,
            url: "https://example.invalid/assets.json".to_string(),
        }
    }

    #[test]
    fn should_include_library_without_rules() {
        let library = sample_library("org.example:lib:1.0.0");
        assert!(VersionManifest::should_include_library_for_current_os(&library));
    }

    #[test]
    fn should_include_library_uses_last_matching_rule() {
        let current_os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };

        let library = Library {
            name: "org.example:lib:1.0.0".to_string(),
            rules: vec![
                Rule {
                    action: "allow".to_string(),
                    os: Some(OsRule {
                        name: Some(current_os.to_string()),
                        version: None,
                        arch: None,
                    }),
                },
                Rule {
                    action: "disallow".to_string(),
                    os: Some(OsRule {
                        name: Some(current_os.to_string()),
                        version: None,
                        arch: None,
                    }),
                },
            ],
            natives: None,
            downloads: None,
            url: None,
        };

        assert!(!VersionManifest::should_include_library_for_current_os(&library));
    }

    #[test]
    fn merge_arguments_appends_child_arguments() {
        let parent = Some(Arguments {
            game: vec![ArgumentValue::String("--username".to_string())],
            jvm: vec![ArgumentValue::String("-Xmx2G".to_string())],
        });
        let child = Some(Arguments {
            game: vec![ArgumentValue::String("--version".to_string())],
            jvm: vec![ArgumentValue::String("-Xms1G".to_string())],
        });

        let merged = merge_arguments(parent, child).expect("merged arguments missing");
        assert_eq!(merged.game.len(), 2);
        assert_eq!(merged.jvm.len(), 2);
    }

    #[test]
    fn merge_libraries_overrides_duplicates_and_keeps_order() {
        let mut a = sample_library("org.example:a:1.0");
        a.url = Some("https://example.invalid/a-parent".to_string());

        let mut b_parent = sample_library("org.example:b:1.0");
        b_parent.url = Some("https://example.invalid/b-parent".to_string());

        let mut b_child = sample_library("org.example:b:1.0");
        b_child.url = Some("https://example.invalid/b-child".to_string());

        let parent = vec![a, b_parent];
        let child = vec![b_child, sample_library("org.example:c:1.0")];

        let merged = merge_libraries(parent, child);
        let names: Vec<String> = merged.iter().map(|l| l.name.clone()).collect();

        assert_eq!(
            names,
            vec![
                "org.example:a:1.0".to_string(),
                "org.example:b:1.0".to_string(),
                "org.example:c:1.0".to_string(),
            ]
        );
        assert_eq!(
            merged[1].url.as_deref(),
            Some("https://example.invalid/b-child")
        );
    }

    #[test]
    fn into_resolved_requires_main_class_and_asset_index() {
        let missing_main = VersionManifest {
            id: "test".to_string(),
            main_class: None,
            jar: None,
            downloads: None,
            arguments: None,
            minecraft_arguments: None,
            libraries: Vec::new(),
            asset_index: Some(sample_asset("assets")),
            assets: None,
            version_type: None,
            inherits_from: None,
        };
        assert!(missing_main
            .into_resolved_with_client_jar("test".to_string())
            .is_err());

        let missing_assets = VersionManifest {
            id: "test".to_string(),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            jar: None,
            downloads: None,
            arguments: None,
            minecraft_arguments: None,
            libraries: Vec::new(),
            asset_index: None,
            assets: None,
            version_type: None,
            inherits_from: None,
        };
        assert!(missing_assets
            .into_resolved_with_client_jar("test".to_string())
            .is_err());
    }

    #[test]
    fn resolve_from_file_applies_parent_inheritance() {
        let root = temp_dir("resolve_inheritance");
        let versions_dir = root.join("versions");
        let parent_dir = versions_dir.join("parent");
        let child_dir = versions_dir.join("child");
        std::fs::create_dir_all(&parent_dir).expect("failed to create parent dir");
        std::fs::create_dir_all(&child_dir).expect("failed to create child dir");

        let parent_json = serde_json::json!({
            "id": "parent",
            "mainClass": "net.minecraft.client.main.Main",
            "assetIndex": {
                "id": "parent-assets",
                "sha1": "deadbeef",
                "size": 1,
                "totalSize": 1,
                "url": "https://example.invalid/parent-assets.json"
            },
            "libraries": [
                { "name": "org.example:parent-lib:1.0" }
            ]
        });

        let child_json = serde_json::json!({
            "id": "child",
            "inheritsFrom": "parent",
            "libraries": [
                { "name": "org.example:child-lib:1.0" }
            ]
        });

        let parent_path = parent_dir.join("parent.json");
        let child_path = child_dir.join("child.json");
        std::fs::write(&parent_path, serde_json::to_vec(&parent_json).unwrap())
            .expect("failed to write parent manifest");
        std::fs::write(&child_path, serde_json::to_vec(&child_json).unwrap())
            .expect("failed to write child manifest");

        let resolved =
            VersionManifest::resolve_from_file(&child_path).expect("failed to resolve manifest");

        assert_eq!(resolved.main_class, "net.minecraft.client.main.Main");
        assert_eq!(resolved.client_jar_id, "parent");
        assert_eq!(resolved.asset_index.id, "parent-assets");
        assert_eq!(resolved.libraries.len(), 2);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_from_file_uses_child_overrides_when_present() {
        let root = temp_dir("resolve_child_override");
        let versions_dir = root.join("versions");
        let parent_dir = versions_dir.join("base");
        let child_dir = versions_dir.join("child");
        std::fs::create_dir_all(&parent_dir).expect("failed to create parent dir");
        std::fs::create_dir_all(&child_dir).expect("failed to create child dir");

        let parent_json = serde_json::json!({
            "id": "base",
            "mainClass": "net.minecraft.client.main.ParentMain",
            "assetIndex": {
                "id": "base-assets",
                "sha1": "deadbeef",
                "size": 1,
                "totalSize": 1,
                "url": "https://example.invalid/base-assets.json"
            }
        });

        let child_json = serde_json::json!({
            "id": "child",
            "inheritsFrom": "base",
            "mainClass": "net.minecraft.client.main.ChildMain",
            "assetIndex": {
                "id": "child-assets",
                "sha1": "cafebabe",
                "size": 2,
                "totalSize": 2,
                "url": "https://example.invalid/child-assets.json"
            }
        });

        let parent_path = parent_dir.join("base.json");
        let child_path = child_dir.join("child.json");
        std::fs::write(&parent_path, serde_json::to_vec(&parent_json).unwrap())
            .expect("failed to write parent manifest");
        std::fs::write(&child_path, serde_json::to_vec(&child_json).unwrap())
            .expect("failed to write child manifest");

        let resolved =
            VersionManifest::resolve_from_file(&child_path).expect("failed to resolve manifest");

        assert_eq!(resolved.main_class, "net.minecraft.client.main.ChildMain");
        assert_eq!(resolved.asset_index.id, "child-assets");

        let _ = std::fs::remove_dir_all(&root);
    }
}
