use crate::resource_downloader::domain::ResourceType;
use std::path::PathBuf;

pub struct GameDetection {}

impl GameDetection {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_default_minecraft_download_dir(&self, resource_type: ResourceType) -> PathBuf {
        let subfolder_target = resource_type.game_folder();

        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let path = PathBuf::from(appdata)
                    .join(".minecraft")
                    .join(subfolder_target);
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                return path;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                let path = PathBuf::from(home)
                    .join("Library/Application Support/minecraft")
                    .join(subfolder_target);
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                return path;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(home) = std::env::var("HOME") {
                let home_path = PathBuf::from(&home);

                let flatpak_path = home_path
                    .join(".var/app/net.minecraft.launcher/data/.minecraft")
                    .join(&subfolder_target);
                if flatpak_path.exists() || Self::is_running_in_flatpak() {
                    let _ = std::fs::create_dir_all(&flatpak_path);
                    return flatpak_path;
                }

                let snap_path = home_path
                    .join("snap/minecraft-launcher/common/.minecraft")
                    .join(&subfolder_target);
                if snap_path.exists() || Self::is_running_in_snap() {
                    let _ = std::fs::create_dir_all(&snap_path);
                    return snap_path;
                }

                let path = home_path.join(".minecraft").join(&subfolder_target);
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                return path;
            }
        }

        dirs::download_dir().expect("Failed to get download dir")
    }

    #[cfg(target_os = "linux")]
    fn is_running_in_flatpak() -> bool {
        std::path::Path::new("/.flatpak-info").exists()
    }

    #[cfg(target_os = "linux")]
    fn is_running_in_snap() -> bool {
        std::env::var("SNAP").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_downloader::domain::ResourceType;

    #[test]
    fn test_get_default_minecraft_download_dir_returns_path() {
        let game_detection = GameDetection::new();

        let resource_types = vec![
            ResourceType::ResourcePack,
            ResourceType::Mod,
            ResourceType::Shader,
        ];

        for resource_type in resource_types {
            let path = game_detection.get_default_minecraft_download_dir(resource_type);
            println!("Resource Type: {:?}", resource_type);
            println!("Detected Path: {}", path.display());
            println!("OS: {}", std::env::consts::OS);

            let expected_subfolder = resource_type.game_folder();

            assert!(
                path.ends_with(&expected_subfolder),
                "Path {:?} should end with {:?}",
                path,
                expected_subfolder
            );
            println!(
                "✓ Path structure is correct (ends with: {})",
                expected_subfolder
            );

            if path.exists() {
                assert!(path.is_dir(), "Path should be a directory: {:?}", path);
                println!("✓ Folder exists and is a directory\n");
            } else {
                println!("ℹ️  Folder does not exist on this system\n");
                println!(
                    "   Reason: This is expected behavior on different systems/configurations\n"
                );
                println!("   Environment variables checked:");
                println!(
                    "   - APPDATA (Windows): {}",
                    std::env::var("APPDATA").is_ok()
                );
                println!("   - HOME (Linux/macOS): {}", std::env::var("HOME").is_ok());
                #[cfg(target_os = "linux")]
                {
                    println!("   - SNAP (Linux Snap): {}", std::env::var("SNAP").is_ok());
                }
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_flatpak_detection() {
        let is_flatpak = GameDetection::is_running_in_flatpak();
        let is_snap = GameDetection::is_running_in_snap();

        println!("Running in Flatpak: {}", is_flatpak);
        println!("Running in Snap: {}", is_snap);
    }

    #[test]
    fn test_resource_type_subfolder_names() {
        assert_eq!(ResourceType::Mod.game_folder(), "mods");
        assert_eq!(ResourceType::ResourcePack.game_folder(), "resourcepacks");
        assert_eq!(ResourceType::Shader.game_folder(), "shaderpacks");

        println!("✓ All resource type subfolder names are correct");
    }

    #[test]
    fn test_path_structure_contains_subfolder() {
        let game_detection = GameDetection::new();

        let test_cases = vec![
            (ResourceType::Mod, "mods"),
            (ResourceType::ResourcePack, "resourcepacks"),
            (ResourceType::Shader, "shaderpacks"),
        ];

        for (resource_type, expected_subfolder) in test_cases {
            let path = game_detection.get_default_minecraft_download_dir(resource_type);

            assert!(
                path.ends_with(expected_subfolder),
                "Path {:?} does not end with '{}' (this is a bug in the logic, not system-dependent)",
                path,
                expected_subfolder
            );

            println!(
                "✓ {} correctly ends with '{}'",
                path.display(),
                expected_subfolder
            );
        }
    }

    #[test]
    #[ignore]
    fn test_system_dependent_folder_creation() {
        // only use this test on a real system, as it depends on actual environment variables
        let game_detection = GameDetection::new();
        let path = game_detection.get_default_minecraft_download_dir(ResourceType::Mod);

        println!("Actual path on this system: {}", path.display());
        println!("Path exists: {}", path.exists());

        if path.exists() {
            assert!(path.is_dir(), "Path should be a directory");
        }
    }
}
