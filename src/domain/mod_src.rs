use super::{MinecraftVersion, ModInfo, ModLoader};
use async_trait::async_trait;

#[async_trait]
pub trait ModProvider: Send + Sync {
    async fn search_mods(
        &self,
        query: &str,
        version: &str,
        loader: &str,
    ) -> anyhow::Result<Vec<ModInfo>>;

    async fn fetch_mod_details(
        &self,
        mod_id: &str,
        version: &str,
        loader: &str,
    ) -> anyhow::Result<ModInfo>;

    async fn get_minecraft_versions(
        &self,
    ) -> anyhow::Result<Vec<MinecraftVersion>>;

    async fn get_mod_loaders(&self) -> anyhow::Result<Vec<ModLoader>>;

    async fn download_mod(
        &self,
        download_url: &str,
        destination: &std::path::Path,
        progress_callback: Box<dyn Fn(f32) + Send>,
    ) -> anyhow::Result<()>;
}