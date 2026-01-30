use crate::resource_downloader::business::cache::{
    ArtifactCallback, ArtifactCommand, ArtifactRequest,
};
use crate::resource_downloader::infra::ConnectionLimiter;
use crate::resource_downloader::infra::adapters::ResourceProviderContext;
use crate::resource_downloader::infra::cache::{ARTIFACT_CACHE_TIME, ARTIFACT_MAX_CONNECTIONS};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tokio::sync::mpsc;

pub struct ArtifactWorker {
    rt_handle: tokio::runtime::Handle,
    cache_dir: PathBuf,
    provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
    connection_limiter: Arc<ConnectionLimiter>,
    in_flight: Arc<RwLock<HashSet<String>>>,
    request_rx: mpsc::Receiver<ArtifactCommand>,
}

impl ArtifactWorker {
    pub fn new(
        rt_handle: tokio::runtime::Handle,
        cache_dir: PathBuf,
        provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
        request_rx: mpsc::Receiver<ArtifactCommand>,
    ) -> Self {
        Self {
            rt_handle,
            cache_dir: cache_dir.join("project").join("artifacts"),
            provider_context_cell,
            connection_limiter: Arc::new(ConnectionLimiter::new(ARTIFACT_MAX_CONNECTIONS)),
            in_flight: Arc::new(RwLock::new(HashSet::new())),
            request_rx,
        }
    }

    pub async fn run(mut self) {
        while let Some(command) = self.request_rx.recv().await {
            match command {
                ArtifactCommand::Fetch(req) => {
                    let key = self.generate_key(&req);

                    if self.in_flight.read().contains(&key) {
                        continue;
                    }

                    self.in_flight.write().insert(key.clone());

                    let cache_dir = self.cache_dir.clone();
                    let provider_context = self.provider_context_cell.get().unwrap().clone();
                    let connection_limiter = self.connection_limiter.clone();
                    let in_flight = Arc::clone(&self.in_flight);

                    let on_complete = req.progress_callback.clone();
                    let on_progress = req.progress_callback.clone();

                    self.rt_handle.spawn(async move {
                        let success = process_download(
                            cache_dir,
                            provider_context,
                            connection_limiter,
                            &key,
                            req,
                            on_progress,
                        )
                        .await;
                        if let Some(cb) = on_complete {
                            cb(Some(success), 1.0);
                        }
                        in_flight.write().remove(&key);
                    });
                }
                ArtifactCommand::Cleanup => {
                    let cache_dir = self.cache_dir.clone();
                    self.rt_handle.spawn(async move {
                        let _ = cleanup_disk_cache(&cache_dir).await;
                    });
                }
            }
        }
    }

    fn generate_key(&self, req: &ArtifactRequest) -> String {
        let mut hasher = DefaultHasher::new();
        req.project.hash(&mut hasher);
        req.version_id.hash(&mut hasher);
        req.artifact_id.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

async fn process_download(
    cache_dir: PathBuf,
    provider_context: ResourceProviderContext,
    connection_limiter: Arc<ConnectionLimiter>,
    key: &str,
    req: ArtifactRequest,
    progress_callback: Option<ArtifactCallback>,
) -> bool {
    let cache_path = cache_dir.join(key);

    if cache_path.exists()
        && let Ok(metadata) = tokio::fs::metadata(&cache_path).await
        && let Ok(modified) = metadata.modified()
        && let Ok(elapsed) = SystemTime::now().duration_since(modified)
        && elapsed > ARTIFACT_CACHE_TIME
    {
        let _ = tokio::fs::remove_file(&cache_path).await;
    }

    let _permit = connection_limiter.acquire(1).await;

    if !cache_path.exists() {
        if let Some(parent) = cache_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let adapter_cb: Box<dyn Fn(f32) + Send> = match progress_callback {
            Some(arc) => Box::new(move |p| arc(None, p)),
            None => Box::new(|_| {}),
        };

        let result = provider_context
            .provider
            .download_project_artifact(
                &provider_context,
                req.project,
                &req.resource_type,
                req.version_id.clone(),
                req.artifact_id.clone(),
                cache_path.clone(),
                adapter_cb,
            )
            .await;

        if result.is_err() {
            return false;
        }
    } else if let Some(cb) = progress_callback {
        cb(None, 1.0);
    }

    if let Some(parent) = req.target_destination.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    tokio::fs::copy(&cache_path, &req.target_destination)
        .await
        .is_ok()
}

async fn cleanup_disk_cache(cache_dir: &PathBuf) -> anyhow::Result<()> {
    let mut entries = tokio::fs::read_dir(cache_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file()
            && let Ok(metadata) = tokio::fs::metadata(&path).await
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = SystemTime::now().duration_since(modified)
            && elapsed > ARTIFACT_CACHE_TIME
        {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
    Ok(())
}
