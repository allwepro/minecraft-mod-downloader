use crate::domain::{AppConfig, ModList, ProjectType};

#[derive(Clone)]
pub struct ConfigManager {
    config_dir: std::path::PathBuf,
}

impl ConfigManager {
    pub fn new() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("minecraft-mod-downloader");

        Ok(Self { config_dir })
    }

    pub async fn ensure_dirs(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.config_dir).await?;
        tokio::fs::create_dir_all(self.config_dir.join("lists")).await?;
        Ok(())
    }

    pub fn get_lists_dir(&self) -> std::path::PathBuf {
        self.config_dir.join("lists")
    }

    pub async fn save_list(&self, list: &ModList) -> anyhow::Result<()> {
        let path = self.get_lists_dir().join(format!("{}.toml", list.id));
        let toml_str = toml::to_string_pretty(list)?;
        tokio::fs::write(path, toml_str).await?;
        Ok(())
    }

    pub async fn load_all_lists(&self) -> anyhow::Result<Vec<ModList>> {
        let mut lists = Vec::new();
        let mut dir = tokio::fs::read_dir(self.get_lists_dir()).await?;

        while let Some(entry) = dir.next_entry().await? {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("toml")
                && let Ok(content) = tokio::fs::read_to_string(entry.path()).await
                && let Ok(list) = toml::from_str::<ModList>(&content)
            {
                lists.push(list);
            }
        }

        Ok(lists)
    }

    pub async fn delete_list(&self, list_id: &str) -> anyhow::Result<()> {
        let path = self.get_lists_dir().join(format!("{list_id}.toml"));
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    pub async fn save_config(&self, config: &AppConfig) -> anyhow::Result<()> {
        let path = self.config_dir.join("config.toml");
        let toml_str = toml::to_string_pretty(config)?;
        tokio::fs::write(path, toml_str).await?;
        Ok(())
    }

    pub async fn load_config(&self) -> anyhow::Result<AppConfig> {
        let path = self.config_dir.join("config.toml");
        let content = tokio::fs::read_to_string(path).await?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn config_exists(&self) -> bool {
        self.config_dir.join("config.toml").exists()
    }

    pub async fn create_default_config(&self) -> anyhow::Result<AppConfig> {
        let config = AppConfig {
            current_list_id: None,
            default_list_name: "New List".to_string(),
        };
        self.save_config(&config).await?;
        Ok(config)
    }

    pub fn get_cache_dir(&self) -> std::path::PathBuf {
        self.config_dir.clone().join("cache")
    }

    pub fn get_default_minecraft_download_dir(
        project_type: ProjectType,
    ) -> Option<std::path::PathBuf> {
        let subfolder = match project_type {
            ProjectType::Mod => "mods",
            ProjectType::ResourcePack => "resourcepacks",
            ProjectType::Shader => "shaderpacks",
            ProjectType::Datapack | ProjectType::Plugin => return None,
        };

        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let path = std::path::PathBuf::from(appdata)
                    .join(".minecraft")
                    .join(subfolder);
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                return Some(path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                let path = std::path::PathBuf::from(home)
                    .join("Library/Application Support/minecraft")
                    .join(subfolder);
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                return Some(path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::PathBuf::from(&home);

                let flatpak_path = home_path
                    .join(".var/app/net.minecraft.launcher/data/.minecraft")
                    .join(subfolder);
                if flatpak_path.exists() || Self::is_running_in_flatpak() {
                    let _ = std::fs::create_dir_all(&flatpak_path);
                    return Some(flatpak_path);
                }

                let snap_path = home_path
                    .join("snap/minecraft-launcher/common/.minecraft")
                    .join(subfolder);
                if snap_path.exists() || Self::is_running_in_snap() {
                    let _ = std::fs::create_dir_all(&snap_path);
                    return Some(snap_path);
                }

                let path = home_path.join(".minecraft").join(subfolder);
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                return Some(path);
            }
        }

        None
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
