use crate::resource_downloader::business::services::ApiService;
use crate::resource_downloader::business::{Event, InternalEvent};
use crate::resource_downloader::domain::ResourceType::Mod;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, Project, ProjectList, ProjectLnk, ProjectTypeConfig,
};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct LegacyListService {
    api_service: Arc<ApiService>,
}

impl LegacyListService {
    pub fn new(api_service: Arc<ApiService>) -> Self {
        Self { api_service }
    }

    pub async fn import_legacy_list(
        &self,
        path: PathBuf,
        list_version: &GameVersion,
        list_loader: &GameLoader,
        default_download_dir: String,
        tx: mpsc::Sender<InternalEvent>,
    ) -> anyhow::Result<ProjectList> {
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read legacy file: {e}"))?;

        let slugs: Vec<String> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect();

        let mut successful_projects = Vec::new();
        let mut failed_slugs = Vec::new();

        for (idx, slug) in slugs.iter().enumerate() {
            let _ = tx
                .send(InternalEvent::Standard(Event::LegacyListProgress {
                    import: true,
                    path: path.clone(),
                    current: idx + 1,
                    total: slugs.len(),
                    message: format!("Resolving '{slug}'..."),
                }))
                .await;

            match self
                .api_service
                .rt_project_pool
                .get_project_by_slug_blocking(slug.clone(), Mod)
                .await
            {
                Ok(Some(project)) => {
                    match self
                        .api_service
                        .rt_project_pool
                        .get_metadata_blocking(project.clone(), Mod)
                        .await
                    {
                        Ok(Some(data)) => successful_projects.push((project, data)),
                        _ => failed_slugs.push(slug.clone()),
                    }
                }
                _ => failed_slugs.push(slug.clone()),
            }
        }

        let mut target_list = ProjectList::new(
            ProjectList::generate_id(),
            format!("Mods of {}", path.file_stem().unwrap().to_str().unwrap()),
            list_version.clone(),
        );

        target_list.set_resource_type(
            Mod,
            ProjectTypeConfig::new(list_loader.clone(), default_download_dir),
        );

        for (project, rtp) in successful_projects {
            target_list.add_project(Project::new_from_rt_project(project, Mod, true, rtp));
        }

        Ok(target_list)
    }

    pub async fn export_legacy_list(
        &self,
        path: PathBuf,
        list_arc: Arc<RwLock<ProjectList>>,
        tx: mpsc::Sender<InternalEvent>,
    ) -> anyhow::Result<Vec<ProjectLnk>> {
        let mut successful_slugs = Vec::new();
        let mut failed_links = Vec::new();

        let (projects, _) = {
            let guard = list_arc.read();
            (guard.get_projects(), guard.get_lnk())
        };

        for (idx, project) in projects.iter().enumerate() {
            let _ = tx
                .send(InternalEvent::Standard(Event::LegacyListProgress {
                    import: false,
                    path: path.clone(),
                    current: idx + 1,
                    total: projects.len(),
                    message: format!("Resolving '{project}'..."),
                }))
                .await;

            match self
                .api_service
                .rt_project_pool
                .get_metadata_blocking(project.clone(), Mod)
                .await
            {
                Ok(Some(data)) if !data.slug.is_empty() => {
                    successful_slugs.push(data.slug);
                }
                _ => failed_links.push(project.clone()),
            }
        }

        let content = format!(
            "# Minecraft Mod List\n# Generated on {}\n\n{}\n# Exported via Flux Launcher\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            successful_slugs.join("\n")
        );

        let temp_path = path.with_extension("mods.tmp");
        tokio::fs::write(&temp_path, content).await?;
        tokio::fs::rename(temp_path, &path).await?;

        Ok(failed_links)
    }
}
