use crate::resource_downloader::domain::{ProjectLnk, ResourceType};
use crate::resource_downloader::infra::adapters::ResourceProviderContext;
use crate::resource_downloader::infra::cache::ArtifactWorker;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;

pub enum ArtifactCommand {
    Fetch(ArtifactRequest),
    Cleanup,
}

pub type ArtifactCallback = Arc<dyn Fn(Option<bool>, f32) + Send + Sync>;
pub struct ArtifactRequest {
    pub project: ProjectLnk,
    pub resource_type: ResourceType,
    pub version_id: String,
    pub artifact_id: String,
    pub target_destination: PathBuf,
    pub progress_callback: Option<ArtifactCallback>,
}

pub struct ArtifactManager {
    request_tx: mpsc::Sender<ArtifactCommand>,
}

impl ArtifactManager {
    pub fn new(
        rt_handle: tokio::runtime::Handle,
        cache_dir: PathBuf,
        provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
    ) -> (Self, ArtifactWorker) {
        let (request_tx, req_rx) = mpsc::channel(100);

        let worker = ArtifactWorker::new(rt_handle, cache_dir, provider_context_cell, req_rx);

        (Self { request_tx }, worker)
    }

    pub fn queue_download(&self, req: ArtifactRequest) {
        let _ = self
            .request_tx
            .try_send(ArtifactCommand::Fetch(req))
            .err()
            .map(|e| {
                log::error!("Failed to send cache request: {e}");
            });
    }

    pub fn cleanup(&self) {
        let _ = self
            .request_tx
            .try_send(ArtifactCommand::Cleanup)
            .err()
            .map(|e| {
                log::error!("Failed to send cleanup request: {e}");
            });
    }
}
