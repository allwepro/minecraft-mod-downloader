use crate::domain::JavaInstallation;
use std::path::PathBuf;
use std::process::Command;

pub struct JavaDetector;

impl JavaDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect all Java installations on the system
    pub fn detect_java_installations() -> Vec<JavaInstallation> {
        let mut installations = Vec::new();

        // Check common Java installation paths based on OS
        let search_paths = Self::get_search_paths();

        for path in search_paths {
            if let Some(installation) = Self::check_java_at_path(&path) {
                installations.push(installation);
            }
        }

        // Also check JAVA_HOME environment variable
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java_path = PathBuf::from(java_home).join("bin").join(Self::java_executable());
            if let Some(installation) = Self::check_java_at_path(&java_path) {
                installations.push(installation);
            }
        }

        // Check java in PATH
        if let Some(installation) = Self::check_java_in_path() {
            installations.push(installation);
        }

        // Remove duplicates based on path
        installations.sort_by(|a, b| a.path.cmp(&b.path));
        installations.dedup_by(|a, b| a.path == b.path);

        installations
    }

    /// Get OS-specific search paths for Java
    fn get_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if cfg!(target_os = "windows") {
            // Windows paths
            paths.push(PathBuf::from("C:\\Program Files\\Java"));
            paths.push(PathBuf::from("C:\\Program Files (x86)\\Java"));
            paths.push(PathBuf::from("C:\\Program Files\\Eclipse Adoptium"));
            paths.push(PathBuf::from("C:\\Program Files\\Microsoft\\jdk"));
        } else if cfg!(target_os = "macos") {
            // macOS paths
            paths.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Library/Java/JavaVirtualMachines"));
            }
        } else {
            // Linux paths
            paths.push(PathBuf::from("/usr/lib/jvm"));
            paths.push(PathBuf::from("/usr/java"));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(".jdks"));
            }
        }

        // Expand directories to find actual java executables
        Self::expand_search_paths(paths)
    }

    /// Recursively search directories for java executables
    fn expand_search_paths(base_paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut java_paths = Vec::new();

        for base in base_paths {
            if !base.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Look for bin/java or bin/java.exe
                        let java_bin = path.join("bin").join(Self::java_executable());
                        if java_bin.exists() {
                            java_paths.push(java_bin);
                        }
                    }
                }
            }
        }

        java_paths
    }

    /// Get the java executable name based on OS
    fn java_executable() -> &'static str {
        if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        }
    }

    /// Check if java exists in PATH
    fn check_java_in_path() -> Option<JavaInstallation> {
        // Try to find the actual path of java using the which crate
        let java_path = match which::which(Self::java_executable()) {
            Ok(path) => path,
            Err(_) => return None,
        };

        let output = Command::new(&java_path)
            .arg("-version")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let version_output = String::from_utf8_lossy(&output.stderr);
        let version = Self::parse_java_version(&version_output)?;

        Some(JavaInstallation {
            path: java_path,
            version,
            is_valid: true,
        })
    }

    /// Check if a path contains a valid Java installation
    fn check_java_at_path(path: &PathBuf) -> Option<JavaInstallation> {
        if !path.exists() {
            return None;
        }

        let output = Command::new(path)
            .arg("-version")
            .output()
            .ok()?;

        if !output.status.success() {
            return Some(JavaInstallation {
                path: path.clone(),
                version: "Unknown".to_string(),
                is_valid: false,
            });
        }

        let version_output = String::from_utf8_lossy(&output.stderr);
        let version = Self::parse_java_version(&version_output)?;

        Some(JavaInstallation {
            path: path.clone(),
            version,
            is_valid: true,
        })
    }

    /// Parse Java version from java -version output
    fn parse_java_version(output: &str) -> Option<String> {
        // Java version output looks like:
        // openjdk version "17.0.1" 2021-10-19
        // or
        // java version "1.8.0_301"

        for line in output.lines() {
            if line.contains("version") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        return Some(line[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }

        None
    }

    /// Find the best Java installation for Minecraft (Java 17+ preferred for modern versions)
    pub fn find_best_java(installations: &[JavaInstallation]) -> Option<&JavaInstallation> {
        installations
            .iter()
            .filter(|i| i.is_valid)
            .max_by_key(|i| Self::java_version_priority(&i.version))
    }

    /// Assign priority to Java versions (higher is better)
    fn java_version_priority(version: &str) -> i32 {
        // Extract major version number
        if let Some(major) = Self::extract_major_version(version) {
            major
        } else {
            0
        }
    }

    /// Extract major version number from version string
    fn extract_major_version(version: &str) -> Option<i32> {
        // Handle both "17.0.1" and "1.8.0_301" formats
        let parts: Vec<&str> = version.split('.').collect();

        if let Some(first) = parts.first() {
            if let Ok(major) = first.parse::<i32>() {
                if major > 1 {
                    return Some(major);
                } else if parts.len() > 1 {
                    // Old format like "1.8.0"
                    return parts[1].parse::<i32>().ok();
                }
            }
        }

        None
    }
}
