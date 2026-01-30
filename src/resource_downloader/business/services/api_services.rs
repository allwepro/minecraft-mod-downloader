use crate::resource_downloader::business::cache::ArtifactManager;
use crate::resource_downloader::business::services::xpool::{
    GameLoaderPool, GameVersionPool, ProjectIconPool, RTProjectPool,
};
use crate::resource_downloader::business::xcache::CoreCacheManager;
use crate::resource_downloader::domain::{GameLoader, ProjectLnk, ResourceType};
use crate::resource_downloader::infra::adapters::{
    ModrinthProvider, ResourceProvider, ResourceProviderContext,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

pub type BoxedWorker = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type UpdateFn = Box<dyn Fn(&egui::Context) + Send + Sync>;

type CleanupFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type CleanupFn = Box<dyn Fn() -> CleanupFuture + Send + Sync>;

#[derive(Clone)]
pub struct ApiService {
    provider: Arc<dyn ResourceProvider>,
    modrinth_provider: Arc<ModrinthProvider>,

    // Pools (Domain APIs)
    pub game_version_pool: Arc<GameVersionPool>,
    pub game_loader_pool: Arc<GameLoaderPool>,
    pub rt_project_pool: Arc<RTProjectPool>,
    pub icon_pool: Arc<ProjectIconPool>,

    // Other
    pub artifact_cache: Arc<ArtifactManager>,

    provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
}

impl ApiService {
    pub fn new(
        rt_handle: &tokio::runtime::Handle,
        cache_dir: PathBuf,
    ) -> (Self, Vec<BoxedWorker>, CleanupFn, UpdateFn) {
        let modrinth_provider = Arc::new(ModrinthProvider::new(Self::get_user_agent()));
        let provider: Arc<dyn ResourceProvider> = modrinth_provider.clone();
        let provider_context_cell = Arc::new(OnceLock::new());

        // 1. Initialize Core Manager & Worker
        let (core_cache, core_worker) = CoreCacheManager::new(
            rt_handle.clone(),
            cache_dir.clone(),
            provider_context_cell.clone(),
        );

        // 2. Initialize Artifact Manager & Worker
        let (artifact_cache, artifact_worker) = ArtifactManager::new(
            rt_handle.clone(),
            cache_dir.clone(),
            provider_context_cell.clone(),
        );

        // 3. Initialize Pools
        let rt_project_pool = Arc::new(RTProjectPool::new(core_cache.clone()));
        let game_version_pool = Arc::new(GameVersionPool::new(core_cache.clone()));
        let game_loader_pool = Arc::new(GameLoaderPool::new(core_cache.clone()));
        let icon_pool = Arc::new(ProjectIconPool::new(core_cache.clone()));
        let artifact_cache_arc = Arc::new(artifact_cache);

        let update_pool = Arc::clone(&icon_pool);
        let update_fn: UpdateFn = Box::new(move |ctx| {
            update_pool.update(ctx);
        });

        // 4. Setup Workers
        let workers: Vec<BoxedWorker> =
            vec![Box::pin(core_worker.run()), Box::pin(artifact_worker.run())];

        // 5. Cleanup Callback
        let core_cache_clone = core_cache.clone();
        let artifact_cache_clone = artifact_cache_arc.clone();

        let clean_expired_callback = move || {
            let core = core_cache_clone.clone();
            let art = artifact_cache_clone.clone();
            Box::pin(async move {
                core.cleanup();
                art.cleanup();
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        };

        let mut service = Self {
            provider,
            modrinth_provider,
            game_version_pool,
            game_loader_pool,
            rt_project_pool,
            icon_pool,
            artifact_cache: artifact_cache_arc,
            provider_context_cell,
        };

        service.set_provider_context();

        (
            service,
            workers,
            Box::new(clean_expired_callback),
            update_fn,
        )
    }

    pub fn get_project_link(&self, project: &ProjectLnk, resource_type: &ResourceType) -> String {
        self.provider
            .get_project_link(resource_type, project.clone())
    }

    pub async fn fetch_modrinth_collection(
        &self,
        collection_id: String,
    ) -> anyhow::Result<(
        String,
        HashMap<ResourceType, (String, GameLoader)>,
        Vec<(String, String, ResourceType)>,
    )> {
        self.modrinth_provider
            .fetch_collection(&self.provider_context(), &collection_id)
            .await
    }

    fn provider_context(&self) -> ResourceProviderContext {
        ResourceProviderContext {
            provider: self.provider.clone(),
            game_loader_pool: self.game_loader_pool.clone(),
            rt_project_pool: self.rt_project_pool.clone(),
        }
    }

    fn set_provider_context(&mut self) {
        self.provider_context_cell
            .set(self.provider_context())
            .unwrap_or_else(|_| {
                log::warn!("Provider context was already set.");
            });
    }

    fn get_user_agent() -> String {
        format!("FluxLauncher/{}", env!("CARGO_PKG_VERSION"))
    }
}
