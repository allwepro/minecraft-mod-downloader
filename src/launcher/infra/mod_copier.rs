use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct ModCopier;

impl ModCopier {
    pub fn new() -> Self {
        Self
    }

    /// Copy mods from source directory to Minecraft mods folder
    /// Returns list of successfully copied mod filenames
    pub async fn copy_mods_to_minecraft(
        source_dir: &Path,
        minecraft_mods_dir: &Path,
        mod_names: &[String],
    ) -> Result<Vec<String>> {
        // Ensure destination directory exists
        tokio::fs::create_dir_all(minecraft_mods_dir)
            .await
            .context("Failed to create mods directory")?;

        let mut copied_mods = Vec::new();

        for mod_name in mod_names {
            // Find the mod file in source directory
            if let Some(mod_file) = Self::find_mod_file(source_dir, mod_name).await? {
                let file_name = mod_file
                    .file_name()
                    .context("Invalid mod filename")?
                    .to_string_lossy()
                    .to_string();

                let metadata = tokio::fs::metadata(&mod_file)
                    .await
                    .context("Failed to get file metadata")?;

                if metadata.len() == 0 {
                    log::warn!("Skipping empty mod file: {}", file_name);
                    continue;
                }

                let dest_path = minecraft_mods_dir.join(&file_name);

                // Copy the file
                match tokio::fs::copy(&mod_file, &dest_path).await {
                    Ok(_) => {
                        log::info!("Copied mod: {} to {}", file_name, dest_path.display());
                        copied_mods.push(file_name);
                    }
                    Err(e) => {
                        log::warn!("Failed to copy mod {}: {}", file_name, e);
                    }
                }
            } else {
                log::warn!("Mod file not found for: {}", mod_name);
            }
        }

        Ok(copied_mods)
    }

    /// Find mod file in directory by mod name
    /// Looks for files with the mod name (ignoring spaces and case)
    async fn find_mod_file(dir: &Path, mod_name: &str) -> Result<Option<PathBuf>> {
        if !dir.exists() {
            return Ok(None);
        }

        let normalized_name = mod_name.to_lowercase().replace(' ', "_");

        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();

                    // Check if filename contains the mod name and ends with .jar
                    if file_name_str.contains(&normalized_name) && file_name_str.ends_with(".jar") {
                        return Ok(Some(path));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Clear all mods from Minecraft mods directory
    pub async fn clear_mods_directory(minecraft_mods_dir: &Path) -> Result<usize> {
        if !minecraft_mods_dir.exists() {
            return Ok(0);
        }

        let mut removed_count = 0;
        let mut entries = tokio::fs::read_dir(minecraft_mods_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "jar" {
                        match tokio::fs::remove_file(&path).await {
                            Ok(_) => {
                                removed_count += 1;
                                log::info!("Removed mod: {}", path.display());
                            }
                            Err(e) => {
                                log::warn!("Failed to remove {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(removed_count)
    }

    /// Backup existing mods directory
    pub async fn backup_mods_directory(minecraft_mods_dir: &Path) -> Result<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let parent = minecraft_mods_dir.parent().context("No parent directory")?;

        let backup_dir = parent.join(format!("mods_backup_{}", timestamp));

        if minecraft_mods_dir.exists() {
            Self::copy_directory(minecraft_mods_dir.to_path_buf(), backup_dir.clone()).await?;
            log::info!("Backed up mods to: {}", backup_dir.display());
        }

        Ok(backup_dir)
    }

    /// Recursively copy a directory
    fn copy_directory(
        src: PathBuf,
        dst: PathBuf,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            tokio::fs::create_dir_all(&dst).await?;

            let mut entries = tokio::fs::read_dir(&src).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let dest_path = dst.join(entry.file_name());

                if path.is_dir() {
                    Self::copy_directory(path, dest_path).await?;
                } else {
                    tokio::fs::copy(&path, &dest_path).await?;
                }
            }

            Ok(())
        })
    }
}
