use crate::resource_downloader::business::xcache::CacheContext;
use crate::resource_downloader::business::xcache::{
    AnyCacheData, CACHE_TYPES, CacheCommand, CacheResponse, CacheType, FetchFn,
};
use crate::resource_downloader::infra::ConnectionLimiter;
use crate::resource_downloader::infra::adapters::ResourceProviderContext;
use crate::resource_downloader::infra::cache::time_now;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use tokio::sync::mpsc;

pub struct CoreCacheWorker {
    rt_handle: tokio::runtime::Handle,
    cache_dir: PathBuf,
    provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
    limiters: HashMap<CacheType, Arc<ConnectionLimiter>>,
    in_flight: Arc<RwLock<HashSet<(CacheType, String)>>>,
    request_rx: mpsc::Receiver<CacheCommand>,
    response_tx: mpsc::Sender<CacheResponse>,
}

impl CoreCacheWorker {
    pub(crate) fn new(
        rt_handle: tokio::runtime::Handle,
        cache_dir: PathBuf,
        provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
        request_rx: mpsc::Receiver<CacheCommand>,
        response_tx: mpsc::Sender<CacheResponse>,
    ) -> Self {
        let mut limiters = HashMap::new();
        for ty in CACHE_TYPES {
            limiters.insert(
                ty,
                Arc::new(ConnectionLimiter::new(ty.config().concurrency)),
            );
        }
        Self {
            rt_handle,
            cache_dir,
            provider_context_cell,
            limiters,
            in_flight: Arc::new(RwLock::new(HashSet::new())),
            request_rx,
            response_tx,
        }
    }

    pub async fn run(mut self) {
        while let Some(cmd) = self.request_rx.recv().await {
            match cmd {
                CacheCommand::Fetch { ty, ctx, fetcher } => {
                    let key = ctx.hashed_key(ty);
                    if !self.in_flight.write().insert((ty, key.clone())) {
                        continue;
                    }

                    let w_ctx = self.clone_context();
                    self.rt_handle.spawn(async move {
                        let res = w_ctx
                            .process_lifecycle(ty, ctx.clone(), key.clone(), fetcher)
                            .await;
                        if let Some(e) = w_ctx.response_tx.try_send(res).err() {
                            log::error!("Failed to send cache response: {e}")
                        }
                        w_ctx.in_flight.write().remove(&(ty, key));
                    });
                }
                CacheCommand::Inject { ty, ctx, data } => {
                    let key = ctx.hashed_key(ty);
                    let w_ctx = self.clone_context();

                    self.rt_handle.spawn(async move {
                        let path = w_ctx.get_fragment_path(ty, &key);
                        let _ = w_ctx.write_fragment(ty, &path, data).await;
                    });
                }
                CacheCommand::Discard { ty, ctx } => {
                    let key = ctx.hashed_key(ty);
                    let w_ctx = self.clone_context();

                    self.rt_handle.spawn(async move {
                        let path = w_ctx.get_fragment_path(ty, &key);
                        if path.exists()
                            && let Err(e) = tokio::fs::remove_file(&path).await
                        {
                            log::error!("Failed to discard disk cache at {path:?}: {e}");
                        }
                    });
                }
                CacheCommand::Cleanup => {
                    let dir = self.cache_dir.clone();
                    self.rt_handle.spawn(async move {
                        let _ = cleanup_disk_all(dir).await;
                    });
                }
            }
        }
    }

    fn clone_context(&self) -> WorkerContext {
        let provider_ctx = self
            .provider_context_cell
            .get()
            .expect("ProviderContext not set")
            .clone();
        WorkerContext {
            cache_dir: self.cache_dir.clone(),
            provider_ctx,
            response_tx: self.response_tx.clone(),
            in_flight: self.in_flight.clone(),
            limiters: self.limiters.clone(),
        }
    }
}

struct WorkerContext {
    cache_dir: PathBuf,
    provider_ctx: ResourceProviderContext,
    response_tx: mpsc::Sender<CacheResponse>,
    in_flight: Arc<RwLock<HashSet<(CacheType, String)>>>,
    limiters: HashMap<CacheType, Arc<ConnectionLimiter>>,
}

impl WorkerContext {
    fn get_fragment_path(&self, ty: CacheType, key: &str) -> PathBuf {
        self.cache_dir
            .join(ty.config().sub_dir)
            .join(format!("{key}.bin"))
    }

    async fn write_fragment(
        &self,
        ty: CacheType,
        path: &PathBuf,
        data: AnyCacheData,
    ) -> anyhow::Result<()> {
        let bytes = (ty.encoder().serialize)(data)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, bytes).await?;
        Ok(())
    }
    async fn process_lifecycle(
        &self,
        ty: CacheType,
        ctx: CacheContext,
        key: String,
        fetcher: FetchFn,
    ) -> CacheResponse {
        let config = ty.config();
        let path = self.get_fragment_path(ty, &key);

        // 1. Disk Check
        if let Ok(bytes) = tokio::fs::read(&path).await
            && let Ok(meta) = tokio::fs::metadata(&path).await
        {
            let modified = meta.modified().unwrap_or(SystemTime::now());
            if SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default()
                < config.ttl
                && let Ok(data) = (ty.encoder().deserialize)(&bytes)
            {
                return CacheResponse::Updated {
                    ty,
                    ctx,
                    data,
                    ts: modified
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
            }
        }

        // 2. Network Fetch
        let limiter = match self.limiters.get(&ty) {
            Some(l) => l,
            None => {
                return CacheResponse::FetchFailed {
                    ty,
                    ctx,
                    error: "Limiter configuration missing".to_string(),
                };
            }
        };
        let _permit = limiter.acquire(1).await;
        match fetcher(self.provider_ctx.clone()).await {
            Ok(data) => {
                let now = time_now();
                let _ = self.write_fragment(ty, &path, data.clone()).await;
                CacheResponse::Updated {
                    ty,
                    ctx,
                    data,
                    ts: now,
                }
            }
            Err(e) => CacheResponse::FetchFailed {
                ty,
                ctx,
                error: format!("{e:#}"),
            },
        }
    }
}

async fn cleanup_disk_all(base_dir: PathBuf) -> anyhow::Result<()> {
    for ty in CACHE_TYPES {
        let config = ty.config();
        let path = base_dir.join(config.sub_dir);
        if !path.exists() {
            continue;
        }

        let mut entries = tokio::fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_file() {
                let modified = meta.modified()?;
                if SystemTime::now().duration_since(modified)? > config.ttl {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
    }
    Ok(())
}
