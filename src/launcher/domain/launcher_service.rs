use super::{LaunchConfig, LaunchResult};
use anyhow::{Context, Result};
use std::process::{Child, Command};

pub struct LauncherService;

impl LauncherService {
    pub fn new() -> Self {
        Self
    }

    /// Launch Minecraft with the given configuration
    pub fn launch_minecraft(config: &LaunchConfig) -> Result<LaunchResult> {
        let java_path = &config.profile.java_path;
        let game_dir = &config.profile.game_directory;
        let version = &config.profile.minecraft_version;

        // Validate paths exist
        if !java_path.exists() {
            return Ok(LaunchResult::Failed {
                error: format!("Java not found at: {}", java_path.display()),
            });
        }

        if !game_dir.exists() {
            return Ok(LaunchResult::Failed {
                error: format!("Game directory not found: {}", game_dir.display()),
            });
        }

        // Build JVM arguments
        let mut args = Vec::new();

        // Memory settings
        args.push(format!("-Xms{}M", config.min_memory_mb));
        args.push(format!("-Xmx{}M", config.max_memory_mb));

        // JVM optimizations for Minecraft
        args.push("-XX:+UseG1GC".to_string());
        args.push("-XX:+ParallelRefProcEnabled".to_string());
        args.push("-XX:MaxGCPauseMillis=200".to_string());
        args.push("-XX:+UnlockExperimentalVMOptions".to_string());
        args.push("-XX:+DisableExplicitGC".to_string());

        // Client JAR path
        let client_jar = game_dir
            .join("versions")
            .join(version)
            .join(format!("{}.jar", version));

        if !client_jar.exists() {
            return Ok(LaunchResult::Failed {
                error: format!(
                    "Minecraft version {} not found. Expected jar at: {}",
                    version,
                    client_jar.display()
                ),
            });
        }

        args.push("-jar".to_string());
        args.push(client_jar.to_string_lossy().to_string());

        // Game arguments
        args.push("--username".to_string());
        args.push(config.username.clone());

        args.push("--version".to_string());
        args.push(version.clone());

        args.push("--gameDir".to_string());
        args.push(game_dir.to_string_lossy().to_string());

        args.push("--assetsDir".to_string());
        args.push(game_dir.join("assets").to_string_lossy().to_string());

        // Launch the process
        log::info!("Launching Minecraft with command: {} {}", java_path.display(), args.join(" "));

        match Command::new(java_path)
            .args(&args)
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                log::info!("Minecraft launched successfully with PID: {}", pid);

                // Don't wait for the process, let it run independently
                std::mem::forget(child);

                Ok(LaunchResult::Success { pid })
            }
            Err(e) => {
                let error = format!("Failed to launch Minecraft: {}", e);
                log::error!("{}", error);
                Ok(LaunchResult::Failed { error })
            }
        }
    }

    /// Launch Minecraft and return the child process for monitoring
    pub fn launch_minecraft_monitored(config: &LaunchConfig) -> Result<Child> {
        let java_path = &config.profile.java_path;
        let game_dir = &config.profile.game_directory;
        let version = &config.profile.minecraft_version;

        // Build arguments (same as above but simplified for MVP)
        let mut args = Vec::new();

        args.push(format!("-Xms{}M", config.min_memory_mb));
        args.push(format!("-Xmx{}M", config.max_memory_mb));

        let client_jar = game_dir
            .join("versions")
            .join(version)
            .join(format!("{}.jar", version));

        args.push("-jar".to_string());
        args.push(client_jar.to_string_lossy().to_string());

        args.push("--username".to_string());
        args.push(config.username.clone());

        args.push("--version".to_string());
        args.push(version.clone());

        args.push("--gameDir".to_string());
        args.push(game_dir.to_string_lossy().to_string());

        let child = Command::new(java_path)
            .args(&args)
            .spawn()
            .context("Failed to spawn Minecraft process")?;

        log::info!("Minecraft launched with PID: {}", child.id());

        Ok(child)
    }

    /// Simplified launch for testing (just starts java -version)
    pub fn test_launch(java_path: &std::path::Path) -> Result<bool> {
        let output = Command::new(java_path)
            .arg("-version")
            .output()
            .context("Failed to run java -version")?;

        Ok(output.status.success())
    }
}
