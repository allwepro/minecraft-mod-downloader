// CLIPPY: nested ifs kept separate for readability in the multi-level native-library resolution logic
#![allow(clippy::collapsible_if)]

use anyhow::{Context, Result};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct NativesExtractor;

impl NativesExtractor {
    /// Extract native libraries from JAR files to target directory
    pub fn extract_natives(library_jars: &[PathBuf], natives_dir: &Path) -> Result<()> {
        // Remove old natives directory to ensure clean extraction
        if natives_dir.exists() {
            log::debug!("Removing old natives directory: {}", natives_dir.display());
            fs::remove_dir_all(natives_dir).context("Failed to remove old natives directory")?;
        }

        // Create natives directory
        log::debug!("Creating natives directory: {}", natives_dir.display());
        fs::create_dir_all(natives_dir).context("Failed to create natives directory")?;
        log::debug!("Natives directory created successfully");

        log::info!("Extracting natives to: {}", natives_dir.display());

        for jar_path in library_jars {
            if !jar_path.exists() {
                log::warn!("Native JAR not found: {}", jar_path.display());
                continue;
            }

            log::debug!("Processing JAR: {}", jar_path.display());
            Self::extract_jar_natives(jar_path, natives_dir)?;
        }

        log::info!("Native extraction complete");
        log::debug!("All natives extracted successfully");
        Ok(())
    }

    /// Extract native files from a single JAR
    fn extract_jar_natives(jar_path: &Path, natives_dir: &Path) -> Result<()> {
        let file = fs::File::open(jar_path)
            .context(format!("Failed to open JAR: {}", jar_path.display()))?;

        let mut archive =
            zip::ZipArchive::new(file).context("Failed to read JAR as ZIP archive")?;

        let mut extracted_count = 0;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();

            // Only extract native library files
            if Self::is_native_file(&file_name) {
                let target_path =
                    natives_dir.join(Path::new(&file_name).file_name().unwrap_or_default());

                // Skip if already exists
                if target_path.exists() {
                    log::debug!("  Skipping (already exists): {}", target_path.display());
                    continue;
                }

                // Extract the file
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                fs::write(&target_path, contents)?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = file.unix_mode().unwrap_or(0o755);
                    let perms = fs::Permissions::from_mode(mode);
                    let _ = fs::set_permissions(&target_path, perms);
                }

                log::debug!("  Extracted: {}", target_path.display());
                log::debug!("Extracted native: {}", target_path.display());
                extracted_count += 1;
            }
        }

        log::debug!("  Extracted {} files from this JAR", extracted_count);
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
        native_extensions
            .iter()
            .any(|ext| filename_lower.ends_with(ext))
            && !filename.starts_with("META-INF/")
    }

    /// Get list of native library JARs from manifest
    pub fn get_native_jars(
        manifest: &crate::launcher::domain::ResolvedManifest,
        game_dir: &Path,
    ) -> Vec<PathBuf> {
        let mut native_jars = Vec::new();
        let libraries_dir = game_dir.join("libraries");
        let classifier_suffix = Self::get_natives_classifier_suffix();
        // On Apple Silicon (aarch64) we prefer `natives-macos-arm64` classifiers over the
        // generic `natives-macos` ones when both are present in the manifest.  We pre-build a set
        // of base coordinates (group:artifact:version) that have an arm64 variant so we can skip
        // the x86_64 fallback later in the main loop, avoiding duplicate or incompatible natives.
        let arm64 = cfg!(target_os = "macos") && cfg!(target_arch = "aarch64");
        let mut arm64_coords = std::collections::HashSet::new();

        if arm64 {
            for library in &manifest.libraries {
                if !manifest.should_include_library(library) {
                    continue;
                }
                if library.name.ends_with(":natives-macos-arm64") {
                    let parts: Vec<&str> = library.name.split(':').collect();
                    if parts.len() == 4 {
                        arm64_coords.insert(format!("{}:{}:{}", parts[0], parts[1], parts[2]));
                    }
                }
            }
        }

        log::debug!("=== Searching for native JARs ===");
        log::debug!("Looking for classifier: {}", classifier_suffix);
        if arm64 {
            log::debug!("Will prefer macOS ARM64 natives when available");
        }
        log::debug!("Total libraries in manifest: {}", manifest.libraries.len());

        for library in &manifest.libraries {
            // Skip if library shouldn't be included
            if !manifest.should_include_library(library) {
                continue;
            }

            // Prefer natives mapping + downloads.classifiers (standard Mojang manifests)
            if let Some(natives) = &library.natives {
                if let Some(classifier_template) = natives.get(Self::get_os_key()) {
                    let classifier = Self::substitute_arch(classifier_template);
                    if let Some(downloads) = &library.downloads {
                        if let Some(classifiers) = &downloads.classifiers {
                            if let Some(artifact) = classifiers.get(&classifier) {
                                let native_jar = libraries_dir.join(&artifact.path);
                                log::debug!("Found native library: {}", library.name);
                                log::debug!("  Checking path: {}", native_jar.display());
                                if native_jar.exists() {
                                    log::debug!("  ✓ Exists!");
                                    native_jars.push(native_jar);
                                } else {
                                    log::debug!("  ✗ Not found!");
                                    log::warn!("Native JAR not found: {}", native_jar.display());
                                }
                                continue;
                            }
                        }
                    }
                }
            }

            // Fallback: classifier embedded in name (non-standard)
            if library.name.contains(classifier_suffix) {
                log::debug!("Found native library (fallback): {}", library.name);
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
                if arm64 && classifier == "natives-macos" {
                    let base = format!("{}:{}:{}", parts[0], parts[1], parts[2]);
                    if arm64_coords.contains(&base) {
                        continue;
                    }
                }

                // Build path to native JAR
                let native_jar = libraries_dir
                    .join(&group)
                    .join(artifact)
                    .join(version)
                    .join(format!("{}-{}-{}.jar", artifact, version, classifier));

                log::debug!("  Checking path: {}", native_jar.display());
                if native_jar.exists() {
                    log::debug!("  ✓ Exists!");
                    native_jars.push(native_jar);
                } else {
                    log::debug!("  ✗ Not found!");
                    log::warn!("Native JAR not found: {}", native_jar.display());
                }
            }
        }

        log::debug!("=== Total native JARs found: {} ===", native_jars.len());
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

    fn substitute_arch(template: &str) -> String {
        if !template.contains("${arch}") {
            return template.to_string();
        }

        let arch = if cfg!(target_pointer_width = "32") {
            "32"
        } else {
            "64"
        };

        template.replace("${arch}", arch)
    }
}

#[cfg(test)]
mod tests {
    use super::NativesExtractor;

    fn sample_native_file_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "lwjgl.dll"
        } else if cfg!(target_os = "macos") {
            "liblwjgl.dylib"
        } else {
            "liblwjgl.so"
        }
    }

    #[test]
    fn detects_native_file_for_current_platform() {
        assert!(NativesExtractor::is_native_file(sample_native_file_name()));
    }

    #[test]
    fn ignores_meta_inf_native_entries() {
        let path = format!("META-INF/{}", sample_native_file_name());
        assert!(!NativesExtractor::is_native_file(&path));
    }

    #[test]
    fn ignores_non_native_files() {
        assert!(!NativesExtractor::is_native_file("readme.txt"));
    }

    #[test]
    fn substitute_arch_replaces_placeholder() {
        let expected_arch = if cfg!(target_pointer_width = "32") {
            "32"
        } else {
            "64"
        };

        assert_eq!(
            NativesExtractor::substitute_arch("natives-linux-${arch}"),
            format!("natives-linux-{}", expected_arch)
        );
    }

    #[test]
    fn substitute_arch_keeps_template_without_placeholder() {
        let template = "natives-macos-arm64";
        assert_eq!(NativesExtractor::substitute_arch(template), template);
    }
}
