use crate::launcher::domain::MinecraftInstallation;
use std::path::PathBuf;

pub struct MinecraftDetector;

impl MinecraftDetector {
    /// Detect Minecraft installation directory
    pub fn detect_minecraft() -> Option<MinecraftInstallation> {
        let minecraft_dir = Self::find_minecraft_dir()?;

        if !minecraft_dir.exists() {
            return None;
        }

        let versions_dir = minecraft_dir.join("versions");
        let mods_dir = minecraft_dir.join("mods");

        let available_versions = Self::detect_installed_versions(&versions_dir);

        Some(MinecraftInstallation {
            root_dir: minecraft_dir,
            mods_dir,
            available_versions,
        })
    }

    /// Find the .minecraft directory based on OS
    fn find_minecraft_dir() -> Option<PathBuf> {
        let base_dir = if cfg!(target_os = "windows") {
            // Windows: %APPDATA%\.minecraft
            dirs::data_dir()?.join(".minecraft")
        } else if cfg!(target_os = "macos") {
            // macOS: ~/Library/Application Support/minecraft
            dirs::data_dir()?.join("minecraft")
        } else {
            // Linux: ~/.minecraft
            dirs::home_dir()?.join(".minecraft")
        };

        if base_dir.exists() {
            Some(base_dir)
        } else {
            None
        }
    }

    /// Detect installed Minecraft versions by scanning versions directory
    /// Only returns versions that have both .json and .jar files
    fn detect_installed_versions(versions_dir: &PathBuf) -> Vec<String> {
        let mut versions = Vec::new();

        if !versions_dir.exists() {
            return versions;
        }

        if let Ok(entries) = std::fs::read_dir(versions_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(version_name) = entry.file_name().to_str() {
                        if Self::is_loader_profile(version_name) {
                            continue;
                        }
                        // Check if this directory contains both json and jar files
                        let json_file = entry.path().join(format!("{}.json", version_name));
                        let jar_file = entry.path().join(format!("{}.jar", version_name));

                        // Only include versions with both files
                        if json_file.exists() && jar_file.exists() {
                            versions.push(version_name.to_string());
                        }
                    }
                }
            }
        }

        versions.sort();
        versions.reverse(); // Most recent versions first
        versions
    }

    fn is_loader_profile(version_name: &str) -> bool {
        let lower = version_name.to_lowercase();
        lower.starts_with("fabric-loader-")
            || lower.starts_with("forge-")
            || lower.starts_with("neoforge-")
            || lower.starts_with("quilt-loader-")
    }

    /// Get the default Minecraft directory (creates if doesn't exist)
    pub fn get_or_create_minecraft_dir() -> Option<PathBuf> {
        let minecraft_dir = Self::find_minecraft_dir()?;

        if !minecraft_dir.exists() {
            std::fs::create_dir_all(&minecraft_dir).ok()?;
        }

        Some(minecraft_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::MinecraftDetector;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "mmd_mc_detector_{}_{}_{}",
            name,
            std::process::id(),
            nonce
        ));
        std::fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn create_version(versions_dir: &PathBuf, version: &str, with_json: bool, with_jar: bool) {
        let dir = versions_dir.join(version);
        std::fs::create_dir_all(&dir).expect("failed to create version dir");
        if with_json {
            let json = dir.join(format!("{}.json", version));
            std::fs::write(&json, b"{}").expect("failed to create json");
        }
        if with_jar {
            let jar = dir.join(format!("{}.jar", version));
            std::fs::write(&jar, b"jar").expect("failed to create jar");
        }
    }

    #[test]
    fn is_loader_profile_detects_known_loaders() {
        assert!(MinecraftDetector::is_loader_profile("fabric-loader-0.16.0-1.20.1"));
        assert!(MinecraftDetector::is_loader_profile("forge-47.2.0"));
        assert!(MinecraftDetector::is_loader_profile("neoforge-20.6.1"));
        assert!(MinecraftDetector::is_loader_profile("quilt-loader-0.26.1"));
        assert!(!MinecraftDetector::is_loader_profile("1.20.1"));
    }

    #[test]
    fn detect_installed_versions_filters_incomplete_and_loader_profiles() {
        let root = temp_dir("detect_versions");
        let versions_dir = root.join("versions");
        std::fs::create_dir_all(&versions_dir).expect("failed to create versions dir");

        create_version(&versions_dir, "1.20.1", true, true);
        create_version(&versions_dir, "1.21.1", true, true);
        create_version(&versions_dir, "1.19.4", true, false);
        create_version(&versions_dir, "fabric-loader-0.16.0-1.20.1", true, true);

        let versions = MinecraftDetector::detect_installed_versions(&versions_dir);

        assert_eq!(versions, vec!["1.21.1".to_string(), "1.20.1".to_string()]);

        let _ = std::fs::remove_dir_all(&root);
    }
}
