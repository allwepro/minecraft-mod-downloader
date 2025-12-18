use crate::domain::{AppConfig, ModList};

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

    pub async fn load_list(&self, list_id: &str) -> anyhow::Result<ModList> {
        let path = self.get_lists_dir().join(format!("{}.toml", list_id));
        let content = tokio::fs::read_to_string(path).await?;
        let list: ModList = toml::from_str(&content)?;
        Ok(list)
    }

    pub async fn load_all_lists(&self) -> anyhow::Result<Vec<ModList>> {
        let mut lists = Vec::new();
        let mut dir = tokio::fs::read_dir(self.get_lists_dir()).await?;

        while let Some(entry) = dir.next_entry().await? {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                    if let Ok(list) = toml::from_str::<ModList>(&content) {
                        lists.push(list);
                    }
                }
            }
        }

        Ok(lists)
    }

    pub async fn delete_list(&self, list_id: &str) -> anyhow::Result<()> {
        let path = self.get_lists_dir().join(format!("{}.toml", list_id));
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
        let default_download_dir = dirs::download_dir()
            .unwrap_or_else(|| self.config_dir.clone())
            .join("minecraft-mods")
            .to_string_lossy()
            .to_string();

        let config = AppConfig {
            selected_version: "1.21.11".to_string(),
            selected_loader: "fabric".to_string(),
            current_list_id: "main".to_string(),
            download_dir: default_download_dir,
        };
        self.save_config(&config).await?;
        Ok(config)
    }
}