use super::{LaunchConfig, LaunchResult, VersionManifest};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct AdvancedLauncher;

impl AdvancedLauncher {
    /// Launch Minecraft with full version manifest support
    pub fn launch_minecraft(config: &LaunchConfig) -> Result<LaunchResult> {
        let version = &config.profile.minecraft_version;
        let game_dir = &config.profile.game_directory;

        // Load version manifest
        let version_json_path = game_dir
            .join("versions")
            .join(version)
            .join(format!("{}.json", version));

        if !version_json_path.exists() {
            return Ok(LaunchResult::Failed {
                error: format!("Version manifest not found: {}", version_json_path.display()),
            });
        }

        let manifest = VersionManifest::from_file(&version_json_path)
            .context("Failed to parse version manifest")?;

        // Build classpath
        let classpath = Self::build_classpath(&manifest, game_dir, version)?;

        // Build game arguments
        let game_args = Self::build_game_arguments(&manifest, config)?;

        // Build JVM arguments
        let mut jvm_args = Self::build_jvm_arguments(&manifest, config)?;

        // Add classpath
        jvm_args.push("-cp".to_string());
        jvm_args.push(classpath);

        // Add main class
        jvm_args.push(manifest.main_class.clone());

        // Combine all arguments
        let mut all_args = jvm_args;
        all_args.extend(game_args);

        log::info!("Launching Minecraft with {} arguments", all_args.len());
        log::debug!("Java: {}", config.profile.java_path.display());
        log::debug!("Main class: {}", manifest.main_class);
        log::debug!("Arguments: {:?}", all_args);

        // Launch the process
        match Command::new(&config.profile.java_path)
            .args(&all_args)
            .current_dir(game_dir)
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                log::info!("Minecraft launched successfully with PID: {}", pid);
                std::mem::forget(child); // Let it run independently
                Ok(LaunchResult::Success { pid })
            }
            Err(e) => {
                let error = format!("Failed to launch Minecraft: {}", e);
                log::error!("{}", error);
                Ok(LaunchResult::Failed { error })
            }
        }
    }

    /// Build classpath from libraries
    fn build_classpath(
        manifest: &VersionManifest,
        game_dir: &Path,
        version: &str,
    ) -> Result<String> {
        let mut classpath_entries = Vec::new();
        let libraries_dir = game_dir.join("libraries");

        // Add all libraries
        for library in &manifest.libraries {
            if !manifest.should_include_library(library) {
                continue;
            }

            // Parse library name (format: group:artifact:version)
            let parts: Vec<&str> = library.name.split(':').collect();
            if parts.len() != 3 {
                log::warn!("Invalid library name format: {}", library.name);
                continue;
            }

            let group = parts[0].replace('.', "/");
            let artifact = parts[1];
            let lib_version = parts[2];

            // Build library path
            let library_path = libraries_dir
                .join(&group)
                .join(artifact)
                .join(lib_version)
                .join(format!("{}-{}.jar", artifact, lib_version));

            if library_path.exists() {
                classpath_entries.push(library_path.to_string_lossy().to_string());
            } else {
                log::warn!("Library not found: {}", library_path.display());
            }
        }

        // Add the minecraft client jar
        let client_jar = game_dir
            .join("versions")
            .join(version)
            .join(format!("{}.jar", version));

        if client_jar.exists() {
            classpath_entries.push(client_jar.to_string_lossy().to_string());
        } else {
            return Err(anyhow::anyhow!(
                "Minecraft client jar not found: {}",
                client_jar.display()
            ));
        }

        // Join with platform-specific separator
        let separator = if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        };

        Ok(classpath_entries.join(separator))
    }

    /// Build game arguments with variable substitution
    fn build_game_arguments(
        manifest: &VersionManifest,
        config: &LaunchConfig,
    ) -> Result<Vec<String>> {
        let mut args = Vec::new();

        // Handle modern arguments format (1.13+)
        if let Some(ref arguments) = manifest.arguments {
            for arg in &arguments.game {
                match arg {
                    super::version_manifest::ArgumentValue::String(s) => {
                        args.push(Self::substitute_variables(s, config));
                    }
                    super::version_manifest::ArgumentValue::Conditional { rules, value } => {
                        // Check if rules match
                        if Self::check_rules(rules) {
                            match value {
                                super::version_manifest::ArgumentValueInner::String(s) => {
                                    args.push(Self::substitute_variables(s, config));
                                }
                                super::version_manifest::ArgumentValueInner::Array(arr) => {
                                    for s in arr {
                                        args.push(Self::substitute_variables(s, config));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Handle legacy arguments format (pre-1.13)
        else if let Some(ref legacy_args) = manifest.minecraft_arguments {
            for arg in legacy_args.split_whitespace() {
                args.push(Self::substitute_variables(arg, config));
            }
        }
        // Fallback to basic arguments
        else {
            args.extend(vec![
                "--username".to_string(),
                config.username.clone(),
                "--version".to_string(),
                config.profile.minecraft_version.clone(),
                "--gameDir".to_string(),
                config.profile.game_directory.to_string_lossy().to_string(),
                "--assetsDir".to_string(),
                config
                    .profile
                    .game_directory
                    .join("assets")
                    .to_string_lossy()
                    .to_string(),
                "--assetIndex".to_string(),
                manifest.assets.clone(),
            ]);
        }

        Ok(args)
    }

    /// Build JVM arguments
    fn build_jvm_arguments(
        manifest: &VersionManifest,
        config: &LaunchConfig,
    ) -> Result<Vec<String>> {
        let mut args = Vec::new();

        // Memory settings
        args.push(format!("-Xms{}M", config.min_memory_mb));
        args.push(format!("-Xmx{}M", config.max_memory_mb));

        // Handle modern JVM arguments
        if let Some(ref arguments) = manifest.arguments {
            for arg in &arguments.jvm {
                match arg {
                    super::version_manifest::ArgumentValue::String(s) => {
                        args.push(Self::substitute_variables(s, config));
                    }
                    super::version_manifest::ArgumentValue::Conditional { rules, value } => {
                        if Self::check_rules(rules) {
                            match value {
                                super::version_manifest::ArgumentValueInner::String(s) => {
                                    args.push(Self::substitute_variables(s, config));
                                }
                                super::version_manifest::ArgumentValueInner::Array(arr) => {
                                    for s in arr {
                                        args.push(Self::substitute_variables(s, config));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Add basic JVM args for older versions
            args.extend(vec![
                "-Djava.library.path=${natives_directory}".to_string(),
                "-Dminecraft.launcher.brand=minecraft-mod-downloader".to_string(),
                "-Dminecraft.launcher.version=1.0".to_string(),
            ]);
        }

        // Substitute variables in JVM args
        args = args
            .into_iter()
            .map(|arg| Self::substitute_variables(&arg, config))
            .collect();

        Ok(args)
    }

    /// Substitute variables in argument strings
    fn substitute_variables(template: &str, config: &LaunchConfig) -> String {
        let game_dir = config.profile.game_directory.to_string_lossy();
        let natives_dir = config
            .profile
            .game_directory
            .join("versions")
            .join(&config.profile.minecraft_version)
            .join("natives");

        template
            .replace("${auth_player_name}", &config.username)
            .replace("${version_name}", &config.profile.minecraft_version)
            .replace("${game_directory}", &game_dir)
            .replace("${assets_root}", &format!("{}/assets", game_dir))
            .replace("${assets_index_name}", &config.profile.minecraft_version)
            .replace("${auth_uuid}", "00000000-0000-0000-0000-000000000000")
            .replace("${auth_access_token}", "0")
            .replace("${user_type}", "legacy")
            .replace("${version_type}", "release")
            .replace(
                "${natives_directory}",
                &natives_dir.to_string_lossy().to_string(),
            )
            .replace("${launcher_name}", "minecraft-mod-downloader")
            .replace("${launcher_version}", "1.0")
            .replace("${classpath}", "") // Will be added separately
    }

    /// Check if rules match current system
    fn check_rules(rules: &[super::version_manifest::Rule]) -> bool {
        let os_name = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };

        let mut allowed = false;

        for rule in rules {
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
}
