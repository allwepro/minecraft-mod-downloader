pub mod modrinth;

use crate::resource_downloader::business::services::xpool::{GameLoaderPool, RTProjectPool};
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ProjectLnk, RTProjectData, RTProjectVersion, ResourceType,
};
use async_trait::async_trait;
use bytes::Bytes;
pub use modrinth::ModrinthProvider;
use std::path::PathBuf;
use std::sync::Arc;

#[async_trait]
pub trait ResourceProvider: Send + Sync {
    fn get_project_link(&self, resource_type: &ResourceType, project: ProjectLnk) -> String;

    async fn fetch_release_game_versions(
        &self,
        context: &ResourceProviderContext,
    ) -> anyhow::Result<Vec<GameVersion>>;

    async fn fetch_game_loaders_for_resource_type(
        &self,
        context: &ResourceProviderContext,
        resource_type: ResourceType,
    ) -> anyhow::Result<Vec<GameLoader>>;

    async fn search_projects(
        &self,
        context: &ResourceProviderContext,
        query: String,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<Vec<ProjectLnk>>;

    async fn fetch_project_from_slug(
        &self,
        context: &ResourceProviderContext,
        slug: String,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<ProjectLnk>;

    async fn fetch_project_data(
        &self,
        context: &ResourceProviderContext,
        project: ProjectLnk,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<RTProjectData>;

    async fn fetch_project_versions(
        &self,
        context: &ResourceProviderContext,
        project: ProjectLnk,
        resource_type: &ResourceType,
        version: Option<&GameVersion>,
        loader: Option<&GameLoader>,
    ) -> anyhow::Result<Vec<RTProjectVersion>>;

    async fn load_project_icon(
        &self,
        context: &ResourceProviderContext,
        project: ProjectLnk,
    ) -> anyhow::Result<Bytes>;

    #[allow(clippy::too_many_arguments)]
    async fn download_project_artifact(
        &self,
        context: &ResourceProviderContext,
        project: ProjectLnk,
        resource_type: &ResourceType,
        version_id: String,
        artifact_id: String,
        destination: PathBuf,
        progress_callback: Box<dyn Fn(f32) + Send>,
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct ResourceProviderContext {
    pub(crate) provider: Arc<dyn ResourceProvider>,
    pub(crate) rt_project_pool: Arc<RTProjectPool>,
    pub(crate) game_loader_pool: Arc<GameLoaderPool>,
}
