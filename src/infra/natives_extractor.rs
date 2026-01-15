use anyhow::{Context, Result};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct NativesExtractor;

impl NativesExtractor {
    /// Extract native libraries from JAR files to target directory
    pub fn extract_natives(
        library_jars: &[PathBuf],
        natives_dir: &Path,
    ) -> Result<()> {
        // Create natives directory
        fs::create_dir_all(natives_dir)
            .context("Failed to create natives directory")?;

        log::info!("Extracting natives to: {}", natives_dir.display());

        for jar_path in library_jars {
            if !jar_path.exists() {
                log::warn!("Native JAR not found: {}", jar_path.display());
                continue;
            }

            Self::extract_jar_natives(jar_path, natives_dir)?;
        }

        log::info!("Native extraction complete");
        Ok(())
    }

    /// Extract native files from a single JAR
    fn extract_jar_natives(jar_path: &Path, natives_dir: &Path) -> Result<()> {
        let file = fs::File::open(jar_path)
            .context(format!("Failed to open JAR: {}", jar_path.display()))?;

        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read JAR as ZIP archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();

            // Only extract native library files
            if Self::is_native_file(&file_name) {
                let target_path = natives_dir.join(
                    Path::new(&file_name)
                        .file_name()
                        .unwrap_or_default()
                );

                // Skip if already exists
                if target_path.exists() {
                    continue;
                }

                // Extract the file
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                fs::write(&target_path, contents)?;

                log::debug!("Extracted native: {}", target_path.display());
            }
        }

        Ok(())
    }

    /// Check if a file is a native library
    fn is_native_file(filename: &str) -> bool {
        let filename_lower = filename.to_lowercase();

        // Platform-specific native extensions
        let native_extensions = if cfg!(target_os = "windows") {
            vec![".dll"]
        } else if cfg!(target_os = "macos") {
            vec![".dylib", ".jnilib"]
        } else {
            vec![".so"]
        };

        // Check if it's a native file and not in META-INF
        native_extensions.iter().any(|ext| filename_lower.ends_with(ext))
            && !filename.starts_with("META-INF/")
    }

    /// Get list of native library JARs from manifest
    pub fn get_native_jars(
        manifest: &crate::domain::VersionManifest,
        game_dir: &Path,
    ) -> Vec<PathBuf> {
        let mut native_jars = Vec::new();
        let libraries_dir = game_dir.join("libraries");

        for library in &manifest.libraries {
            // Skip if library shouldn't be included
            if !manifest.should_include_library(library) {
                continue;
            }

            // Check if library has natives
            if let Some(ref natives) = library.natives {
                let os_key = Self::get_os_key();

                if let Some(classifier) = natives.get(os_key) {
                    // Parse library name
                    let parts: Vec<&str> = library.name.split(':').collect();
                    if parts.len() != 3 {
                        continue;
                    }

                    let group = parts[0].replace('.', "/");
                    let artifact = parts[1];
                    let version = parts[2];

                    // Build path to native JAR
                    let native_jar = libraries_dir
                        .join(&group)
                        .join(artifact)
                        .join(version)
                        .join(format!("{}-{}-{}.jar", artifact, version, classifier));

                    if native_jar.exists() {
                        native_jars.push(native_jar);
                    } else {
                        log::warn!("Native JAR not found: {}", native_jar.display());
                    }
                }
            }
        }

        native_jars
    }

    /// Get OS key for natives lookup
    fn get_os_key() -> &'static str {
        if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        }
    }
}
