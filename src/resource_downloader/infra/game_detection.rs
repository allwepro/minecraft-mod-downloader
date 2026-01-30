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
