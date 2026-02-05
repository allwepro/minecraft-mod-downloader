use super::{LaunchConfig, LaunchResult, ResolvedManifest, VersionManifest};
use crate::launcher::infra::NativesExtractor;
use anyhow::{Context, Result};
use std::path::Path;
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

        let manifest = VersionManifest::resolve_from_file(&version_json_path)
            .context("Failed to parse/resolve version manifest")?;

        // Extract native libraries
        let natives_dir = game_dir
            .join("versions")
            .join(version)
            .join("natives");

        let native_jars = NativesExtractor::get_native_jars(&manifest, game_dir);
        println!("=== NATIVES DEBUG ===");
        println!("Found {} native JAR files", native_jars.len());
        log::info!("Found {} native JAR files", native_jars.len());

        if native_jars.is_empty() {
            println!("WARNING: No native libraries found! This will likely cause LWJGL errors");
            println!("Checking manifest for natives info...");
            log::warn!("No native libraries found! This will likely cause LWJGL errors");
            log::warn!("Checking manifest for natives info...");
            for (i, lib) in manifest.libraries.iter().enumerate().take(5) {
                println!("  Library {}: {} (rules: {:?})", i, lib.name, lib.rules);
                log::warn!("  Library {}: {} (natives: {:?})", i, lib.name, lib.natives);
            }
        } else {
            println!("Extracting {} native libraries", native_jars.len());
            log::info!("Extracting {} native libraries", native_jars.len());
            for jar in &native_jars {
                println!("  Native JAR: {}", jar.display());
                log::info!("  Native JAR: {}", jar.display());
            }
            NativesExtractor::extract_natives(&native_jars, &natives_dir)
                .context("Failed to extract native libraries")?;
            println!("Native extraction complete!");
        }

        // Build classpath
        let classpath = Self::build_classpath(&manifest, game_dir)?;

        // Build game arguments
        let game_args = Self::build_game_arguments(&manifest, config, &classpath)?;

        // Build JVM arguments with classpath
        let mut jvm_args = Self::build_jvm_arguments(&manifest, config, &classpath)?;

        // Ensure JNA uses a short native directory to avoid path-length issues
        let jna_short_dir = std::env::temp_dir().join(format!("mmd-jna-{}", std::process::id()));
        if let Err(e) = std::fs::create_dir_all(&jna_short_dir) {
            log::warn!(
                "Failed to create JNA short dir {}: {}",
                jna_short_dir.display(),
                e
            );
        }
        Self::ensure_macos_framework_shims(&jna_short_dir);

        if Self::ensure_jna_boot_library(&manifest, game_dir, &jna_short_dir)? {
            let jna_boot = format!("-Djna.boot.library.path={}", jna_short_dir.to_string_lossy());
            Self::replace_or_push_arg(&mut jvm_args, "-Djna.boot.library.path=", jna_boot);
        }

        // Ensure java-objc-bridge native library is present (uses JNA)
        if Self::ensure_objc_bridge_library(&manifest, game_dir, &jna_short_dir)? {
            let jna_path = format!("-Djna.library.path={}", jna_short_dir.to_string_lossy());
            Self::replace_or_push_arg(&mut jvm_args, "-Djna.library.path=", jna_path);
        }

        // Ensure LWJGL uses the extracted natives directory
        let lwjgl_arg = format!(
            "-Dorg.lwjgl.librarypath={}",
            natives_dir.to_string_lossy()
        );
        Self::replace_or_push_arg(&mut jvm_args, "-Dorg.lwjgl.librarypath=", lwjgl_arg);

        // Optional: enable JNA debug logging to identify native load failures
        if std::env::var("MMD_JNA_DEBUG").is_ok() {
            println!("JNA debug enabled (MMD_JNA_DEBUG=1)");
            Self::replace_or_push_arg(
                &mut jvm_args,
                "-Djna.debug_load=",
                "-Djna.debug_load=true".to_string(),
            );
            Self::replace_or_push_arg(
                &mut jvm_args,
                "-Djna.debug_load.jna=",
                "-Djna.debug_load.jna=true".to_string(),
            );
            Self::replace_or_push_arg(
                &mut jvm_args,
                "-Djna.debug=",
                "-Djna.debug=true".to_string(),
            );
        }

        // Add main class
        jvm_args.push(manifest.main_class.clone());

        // Combine all arguments
        let mut all_args = jvm_args.clone();
        all_args.extend(game_args);

        println!("=== LAUNCH ARGUMENTS DEBUG ===");
        println!("Java: {}", config.profile.java_path.display());
        println!("Working directory: {}", game_dir.display());
        println!("Main class: {}", manifest.main_class);
        println!("Total arguments: {}", all_args.len());
        println!("\nJVM Arguments containing 'natives' or 'library.path':");
        for (i, arg) in all_args.iter().enumerate() {
            if arg.contains("natives") || arg.contains("library.path") {
                println!("  [{}] {}", i, arg);
            }
        }
        println!("\nFirst 20 arguments:");
        for (i, arg) in all_args.iter().enumerate().take(20) {
            println!("  [{}] {}", i, arg);
        }

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
        manifest: &ResolvedManifest,
        game_dir: &Path,
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
            .join(&manifest.client_jar_id)
            .join(format!("{}.jar", manifest.client_jar_id));

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
        manifest: &ResolvedManifest,
        config: &LaunchConfig,
        classpath: &str,
    ) -> Result<Vec<String>> {
        let mut args = Vec::new();

        // Handle modern arguments format (1.13+)
        if let Some(ref arguments) = manifest.arguments {
            for arg in &arguments.game {
                match arg {
                    super::version_manifest::ArgumentValue::String(s) => {
                        args.push(Self::substitute_variables(
                            s,
                            config,
                            classpath,
                            &manifest.asset_index.id,
                        ));
                    }
                    super::version_manifest::ArgumentValue::Conditional { rules, value } => {
                        // Check if rules match
                        if Self::check_rules(rules) {
                            match value {
                                super::version_manifest::ArgumentValueInner::String(s) => {
                                    args.push(Self::substitute_variables(
                                        s,
                                        config,
                                        classpath,
                                        &manifest.asset_index.id,
                                    ));
                                }
                                super::version_manifest::ArgumentValueInner::Array(arr) => {
                                    for s in arr {
                                        args.push(Self::substitute_variables(
                                            s,
                                            config,
                                            classpath,
                                            &manifest.asset_index.id,
                                        ));
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
                args.push(Self::substitute_variables(
                    arg,
                    config,
                    classpath,
                    &manifest.asset_index.id,
                ));
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
                manifest.asset_index.id.clone(),
            ]);
        }

        args = args
            .into_iter()
            .map(|arg| Self::normalize_argument(&arg))
            .filter(|arg| !arg.is_empty())
            .collect();

        Ok(args)
    }

    /// Build JVM arguments
    fn build_jvm_arguments(
        manifest: &ResolvedManifest,
        config: &LaunchConfig,
        classpath: &str,
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
                        args.push(Self::substitute_variables(
                            s,
                            config,
                            classpath,
                            &manifest.asset_index.id,
                        ));
                    }
                    super::version_manifest::ArgumentValue::Conditional { rules, value } => {
                        if Self::check_rules(rules) {
                            match value {
                                super::version_manifest::ArgumentValueInner::String(s) => {
                                    args.push(Self::substitute_variables(
                                        s,
                                        config,
                                        classpath,
                                        &manifest.asset_index.id,
                                    ));
                                }
                                super::version_manifest::ArgumentValueInner::Array(arr) => {
                                    for s in arr {
                                        args.push(Self::substitute_variables(
                                            s,
                                            config,
                                            classpath,
                                            &manifest.asset_index.id,
                                        ));
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
                "-cp".to_string(),
                classpath.to_string(),
            ]);
        }

        // Substitute variables in JVM args
        args = args
            .into_iter()
            .map(|arg| {
                let substituted =
                    Self::substitute_variables(&arg, config, classpath, &manifest.asset_index.id);
                Self::normalize_argument(&substituted)
            })
            .filter(|arg| !arg.is_empty())
            .collect();

        Ok(args)
    }

    /// Substitute variables in argument strings
    fn substitute_variables(
        template: &str,
        config: &LaunchConfig,
        classpath: &str,
        asset_index_id: &str,
    ) -> String {
        let game_dir = config.profile.game_directory.to_string_lossy();
        let natives_dir = config
            .profile
            .game_directory
            .join("versions")
            .join(&config.profile.minecraft_version)
            .join("natives");

        if template.contains("${natives_directory}") {
            println!("Substituting natives_directory: {}", natives_dir.display());
        }

        template
            .replace("${auth_player_name}", &config.username)
            .replace("${version_name}", &config.profile.minecraft_version)
            .replace("${game_directory}", &game_dir)
            .replace("${assets_root}", &format!("{}/assets", game_dir))
            .replace("${assets_index_name}", asset_index_id)
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
            .replace("${classpath}", classpath)
    }

    fn normalize_argument(arg: &str) -> String {
        let trimmed = arg.trim();
        if let Some(rest) = trimmed.strip_prefix("-D") {
            if let Some((key, value)) = rest.split_once('=') {
                return format!("-D{}={}", key, value.trim());
            }
        }
        trimmed.to_string()
    }

    fn replace_or_push_arg(args: &mut Vec<String>, prefix: &str, value: String) {
        if let Some(pos) = args.iter().position(|arg| arg.starts_with(prefix)) {
            args[pos] = value;
        } else {
            args.push(value);
        }
    }

    fn ensure_macos_framework_shims(target_dir: &Path) {
        #[cfg(target_os = "macos")]
        {
            let jna_debug = std::env::var_os("MMD_JNA_DEBUG").is_some();
            Self::ensure_macos_framework_shim(
                target_dir,
                "IOKit",
                "/System/Library/Frameworks/IOKit.framework/IOKit",
                jna_debug,
            );
            Self::ensure_macos_framework_shim(
                target_dir,
                "CoreFoundation",
                "/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation",
                jna_debug,
            );
        }
    }

    #[cfg(target_os = "macos")]
    fn ensure_macos_framework_shim(
        target_dir: &Path,
        name: &str,
        framework_path: &str,
        jna_debug: bool,
    ) {
        let dest = target_dir.join(format!("lib{}.dylib", name));
        let needs_build = match std::fs::symlink_metadata(&dest) {
            Ok(meta) => meta.file_type().is_symlink() || meta.len() == 0,
            Err(_) => true,
        };

        if !needs_build {
            return;
        }

        if dest.exists() {
            if let Err(e) = std::fs::remove_file(&dest) {
                log::warn!(
                    "Failed to remove existing {} shim {}: {}",
                    name,
                    dest.display(),
                    e
                );
                if jna_debug {
                    println!(
                        "Failed to remove existing {} shim {}: {}",
                        name,
                        dest.display(),
                        e
                    );
                }
            }
        }

        let stub_source = target_dir.join(format!("mmd_{}_stub.c", name.to_lowercase()));
        if std::fs::write(&stub_source, format!("int mmd_{}_stub(void){{return 0;}}\n", name))
            .is_err()
        {
            log::warn!("Failed to write {} stub source", name);
            return;
        }

        let status = Command::new("clang")
            .args([
                "-dynamiclib",
                "-o",
                dest.to_string_lossy().as_ref(),
                stub_source.to_string_lossy().as_ref(),
                &format!("-Wl,-reexport_library,{}", framework_path),
                &format!("-Wl,-install_name,lib{}.dylib", name),
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                let _ = std::fs::remove_file(&stub_source);
                if jna_debug {
                    let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
                    println!(
                        "Built lib{}.dylib shim at {} ({} bytes)",
                        name,
                        dest.display(),
                        size
                    );
                }
            }
            Ok(s) => {
                log::warn!("Failed to build lib{}.dylib shim (status {})", name, s);
                if jna_debug {
                    println!("{} shim build failed with status {}", name, s);
                }
                Self::fallback_framework_symlink(&dest, framework_path, name, jna_debug);
            }
            Err(e) => {
                log::warn!("Failed to run clang for lib{}.dylib shim: {}", name, e);
                if jna_debug {
                    println!("{} shim build failed to run clang: {}", name, e);
                }
                Self::fallback_framework_symlink(&dest, framework_path, name, jna_debug);
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn fallback_framework_symlink(
        dest: &Path,
        framework_path: &str,
        name: &str,
        jna_debug: bool,
    ) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = Path::new(framework_path);
            if dest.exists() {
                if let Err(e) = std::fs::remove_file(dest) {
                    log::warn!(
                        "Failed to remove existing {} shim {}: {}",
                        name,
                        dest.display(),
                        e
                    );
                    if jna_debug {
                        println!(
                            "Failed to remove existing {} shim {}: {}",
                            name,
                            dest.display(),
                            e
                        );
                    }
                }
            }
            match symlink(target, dest) {
                Ok(_) => {
                    if jna_debug {
                        println!("Created lib{}.dylib symlink to {}", name, target.display());
                    }
                }
                Err(e) => {
                    log::warn!("Failed to create lib{}.dylib symlink: {}", name, e);
                    if jna_debug {
                        println!("{} symlink failed: {}", name, e);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn ensure_macos_framework_shim(
        _target_dir: &Path,
        _name: &str,
        _framework_path: &str,
        _jna_debug: bool,
    ) {
    }

    #[cfg(not(target_os = "macos"))]
    fn fallback_framework_symlink(
        _dest: &Path,
        _framework_path: &str,
        _name: &str,
        _jna_debug: bool,
    ) {
    }

    fn ensure_jna_boot_library(
        manifest: &ResolvedManifest,
        game_dir: &Path,
        target_dir: &Path,
    ) -> Result<bool> {
        let jna_jar = Self::find_jna_jar(manifest, game_dir)?;
        let jna_jar = match jna_jar {
            Some(path) => path,
            None => return Ok(false),
        };

        std::fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create {}", target_dir.display()))?;

        let resource = Self::jna_resource_path();
        let file_name = resource
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid JNA resource path"))?;
        let dest = target_dir.join(file_name);

        if !dest.exists() || dest.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            let file = std::fs::File::open(&jna_jar)
                .with_context(|| format!("Failed to open {}", jna_jar.display()))?;
            let mut archive =
                zip::ZipArchive::new(file).context("Failed to read JNA jar as zip")?;
            let mut entry = archive
                .by_name(&resource)
                .with_context(|| format!("JNA resource not found: {}", resource))?;
            let mut out = std::fs::File::create(&dest)
                .with_context(|| format!("Failed to create {}", dest.display()))?;
            std::io::copy(&mut entry, &mut out)
                .with_context(|| format!("Failed to extract {}", dest.display()))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                let _ = std::fs::set_permissions(&dest, perms);
            }
        }

        Ok(true)
    }

    fn ensure_objc_bridge_library(
        manifest: &ResolvedManifest,
        game_dir: &Path,
        target_dir: &Path,
    ) -> Result<bool> {
        let objc_jar = Self::find_library_jar(manifest, game_dir, "ca.weblite", "java-objc-bridge")?;
        let objc_jar = match objc_jar {
            Some(path) => path,
            None => return Ok(false),
        };

        std::fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create {}", target_dir.display()))?;

        let dest = target_dir.join("libjcocoa.dylib");
        if !dest.exists() || dest.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            let file = std::fs::File::open(&objc_jar)
                .with_context(|| format!("Failed to open {}", objc_jar.display()))?;
            let mut archive =
                zip::ZipArchive::new(file).context("Failed to read java-objc-bridge jar")?;
            let mut entry = archive
                .by_name("libjcocoa.dylib")
                .context("libjcocoa.dylib not found in java-objc-bridge jar")?;
            let mut out = std::fs::File::create(&dest)
                .with_context(|| format!("Failed to create {}", dest.display()))?;
            std::io::copy(&mut entry, &mut out)
                .with_context(|| format!("Failed to extract {}", dest.display()))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                let _ = std::fs::set_permissions(&dest, perms);
            }
        }

        Ok(true)
    }

    fn find_jna_jar(
        manifest: &ResolvedManifest,
        game_dir: &Path,
    ) -> Result<Option<std::path::PathBuf>> {
        Self::find_library_jar(manifest, game_dir, "net.java.dev.jna", "jna")
    }

    fn find_library_jar(
        manifest: &ResolvedManifest,
        game_dir: &Path,
        group: &str,
        artifact: &str,
    ) -> Result<Option<std::path::PathBuf>> {
        let libraries_dir = game_dir.join("libraries");
        let prefix = format!("{}:{}:", group, artifact);

        for library in &manifest.libraries {
            if !manifest.should_include_library(library) {
                continue;
            }
            if !library.name.starts_with(&prefix) {
                continue;
            }

            if let Some(downloads) = &library.downloads {
                if let Some(artifact_info) = &downloads.artifact {
                    return Ok(Some(libraries_dir.join(&artifact_info.path)));
                }
            }

            let parts: Vec<&str> = library.name.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let group_path = parts[0].replace('.', "/");
            let artifact_id = parts[1];
            let version = parts[2];
            let jar = libraries_dir
                .join(group_path)
                .join(artifact_id)
                .join(version)
                .join(format!("{}-{}.jar", artifact_id, version));
            return Ok(Some(jar));
        }

        Ok(None)
    }

    fn jna_resource_path() -> String {
        let os = if cfg!(target_os = "windows") {
            "win32"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x86-64"
        } else if cfg!(target_arch = "x86") {
            "x86"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x86-64"
        };

        let dir = if os == "darwin" && arch == "aarch64" {
            "darwin-aarch64".to_string()
        } else if os == "darwin" && arch == "x86-64" {
            "darwin-x86-64".to_string()
        } else if os == "win32" && arch == "x86-64" {
            "win32-x86-64".to_string()
        } else if os == "win32" && arch == "x86" {
            "win32-x86".to_string()
        } else if os == "linux" && arch == "x86-64" {
            "linux-x86-64".to_string()
        } else if os == "linux" && arch == "aarch64" {
            "linux-aarch64".to_string()
        } else if os == "linux" && arch == "x86" {
            "linux-x86".to_string()
        } else {
            format!("{}-{}", os, arch)
        };

        let file = if cfg!(target_os = "windows") {
            "jnidispatch.dll"
        } else if cfg!(target_os = "macos") {
            "libjnidispatch.jnilib"
        } else {
            "libjnidispatch.so"
        };

        format!("com/sun/jna/{}/{}", dir, file)
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

    fn matches_arch(arch: &str) -> bool {
        match arch {
            "x86" => cfg!(target_pointer_width = "32"),
            "x86_64" => cfg!(target_arch = "x86_64"),
            "arm64" | "aarch64" => cfg!(target_arch = "aarch64"),
            _ => true,
        }
    }
}
