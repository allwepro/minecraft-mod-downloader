use crate::resource_downloader::domain::AppConfig;
use std::path::PathBuf;

pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        self.ensure_dirs().await?;
        if !self.config_exists() {
            self.create_default_config().await?;
        }
        Ok(())
    }

    // Program directory operations
    pub async fn ensure_dirs(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.config_dir).await?;
        tokio::fs::create_dir_all(self.get_lists_dir()).await?;
        tokio::fs::create_dir_all(self.get_cache_dir()).await?;
        Ok(())
    }

    pub fn get_lists_dir(&self) -> PathBuf {
        self.config_dir.join("lists")
    }

    pub fn get_cache_dir(&self) -> PathBuf {
        self.config_dir.clone().join("cache")
    }

    // Config operations
    pub fn config_exists(&self) -> bool {
        self.config_dir.join("config.toml").exists()
    }
    pub async fn load_config(&self) -> anyhow::Result<AppConfig> {
        let path = self.config_dir.join("config.toml");
        let content = tokio::fs::read_to_string(path).await?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }
    pub async fn save_config(&self, config: &AppConfig) -> anyhow::Result<()> {
        let path = self.config_dir.join("config.toml");
        let toml_str = toml::to_string_pretty(config)?;
        tokio::fs::write(path, toml_str).await?;
        Ok(())
    }

    pub async fn create_default_config(&self) -> anyhow::Result<AppConfig> {
        let config = AppConfig {
            last_open_list_id: None,
            default_list_name: "New List".to_string(),
        };
        self.save_config(&config).await?;
        Ok(config)
    }
}
