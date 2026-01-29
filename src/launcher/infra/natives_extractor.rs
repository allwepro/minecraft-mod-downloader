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
        // Remove old natives directory to ensure clean extraction
        if natives_dir.exists() {
            println!("Removing old natives directory: {}", natives_dir.display());
            fs::remove_dir_all(natives_dir)
                .context("Failed to remove old natives directory")?;
        }

        // Create natives directory
        println!("Creating natives directory: {}", natives_dir.display());
        fs::create_dir_all(natives_dir)
            .context("Failed to create natives directory")?;
        println!("Natives directory created successfully");

        log::info!("Extracting natives to: {}", natives_dir.display());

        for jar_path in library_jars {
            if !jar_path.exists() {
                log::warn!("Native JAR not found: {}", jar_path.display());
                continue;
            }

            println!("Processing JAR: {}", jar_path.display());
            Self::extract_jar_natives(jar_path, natives_dir)?;
        }

        log::info!("Native extraction complete");
        println!("All natives extracted successfully");
        Ok(())
    }

    /// Extract native files from a single JAR
    fn extract_jar_natives(jar_path: &Path, natives_dir: &Path) -> Result<()> {
        let file = fs::File::open(jar_path)
            .context(format!("Failed to open JAR: {}", jar_path.display()))?;

        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read JAR as ZIP archive")?;

        let mut extracted_count = 0;
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
                    println!("  Skipping (already exists): {}", target_path.display());
                    continue;
                }

                // Extract the file
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                fs::write(&target_path, contents)?;

                println!("  Extracted: {}", target_path.display());
                log::debug!("Extracted native: {}", target_path.display());
                extracted_count += 1;
            }
        }

        println!("  Extracted {} files from this JAR", extracted_count);
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
        manifest: &crate::launcher::domain::VersionManifest,
        game_dir: &Path,
    ) -> Vec<PathBuf> {
        let mut native_jars = Vec::new();
        let libraries_dir = game_dir.join("libraries");
        let classifier_suffix = Self::get_natives_classifier_suffix();

        println!("=== Searching for native JARs ===");
        println!("Looking for classifier: {}", classifier_suffix);
        println!("Total libraries in manifest: {}", manifest.libraries.len());

        for library in &manifest.libraries {
            // Skip if library shouldn't be included
            if !manifest.should_include_library(library) {
                continue;
            }

            // Check if this is a native library by looking at the name
            // Native libraries have a classifier like "natives-macos", "natives-linux", "natives-windows"

            if library.name.contains(classifier_suffix) {
                println!("Found native library: {}", library.name);
                // Parse library name with classifier
                // Format: "group:artifact:version:classifier"
                let parts: Vec<&str> = library.name.split(':').collect();
                if parts.len() != 4 {
                    continue;
                }

                let group = parts[0].replace('.', "/");
                let artifact = parts[1];
                let version = parts[2];
                let classifier = parts[3];

                // Build path to native JAR
                let native_jar = libraries_dir
                    .join(&group)
                    .join(artifact)
                    .join(version)
                    .join(format!("{}-{}-{}.jar", artifact, version, classifier));

                println!("  Checking path: {}", native_jar.display());
                if native_jar.exists() {
                    println!("  ✓ Exists!");
                    native_jars.push(native_jar);
                } else {
                    println!("  ✗ Not found!");
                    log::warn!("Native JAR not found: {}", native_jar.display());
                }
            }
        }

        println!("=== Total native JARs found: {} ===", native_jars.len());
        native_jars
    }

    /// Get the classifier suffix for native libraries on this OS
    fn get_natives_classifier_suffix() -> &'static str {
        if cfg!(target_os = "windows") {
            "natives-windows"
        } else if cfg!(target_os = "macos") {
            // Check if we're on ARM64 Mac
            if cfg!(target_arch = "aarch64") {
                "natives-macos-arm64"
            } else {
                "natives-macos"
            }
        } else {
            "natives-linux"
        }
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
