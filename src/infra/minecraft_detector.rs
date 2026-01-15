use crate::domain::MinecraftInstallation;
use std::path::PathBuf;

pub struct MinecraftDetector;

impl MinecraftDetector {
    pub fn new() -> Self {
        Self
    }

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
            versions_dir,
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

    /// Ensure mods directory exists
    pub fn ensure_mods_dir(minecraft_dir: &PathBuf) -> std::io::Result<PathBuf> {
        let mods_dir = minecraft_dir.join("mods");
        std::fs::create_dir_all(&mods_dir)?;
        Ok(mods_dir)
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
