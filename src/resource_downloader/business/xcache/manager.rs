use crate::resource_downloader::business::xcache::CacheContext;
use crate::resource_downloader::business::xcache::common::{
    AnyCacheData, CacheCommand, CacheEntry, CacheResponse, CacheType, FetchFn,
};
use crate::resource_downloader::infra::adapters::ResourceProviderContext;
use crate::resource_downloader::infra::cache::time_now;
use crate::resource_downloader::infra::xcache::CoreCacheWorker;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};
use tokio::sync::{Notify, mpsc};

pub type CacheRegistry = Arc<RwLock<HashMap<(CacheType, String), Result<CacheEntry, String>>>>;

pub struct CoreCacheManager {
    registry: CacheRegistry,
    request_tx: mpsc::Sender<CacheCommand>,
    notify: Arc<Notify>,
}

impl CoreCacheManager {
    pub fn new(
        rt_handle: tokio::runtime::Handle,
        cache_dir: PathBuf,
        provider_context_cell: Arc<OnceLock<ResourceProviderContext>>,
    ) -> (Arc<Self>, CoreCacheWorker) {
        let (req_tx, req_rx) = mpsc::channel(100);
        let (res_tx, mut res_rx) = mpsc::channel(100);
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let notify = Arc::new(Notify::new());

        let manager = Arc::new(Self {
            registry: Arc::clone(&registry),
            request_tx: req_tx,
            notify: Arc::clone(&notify),
        });

        let worker = CoreCacheWorker::new(
            rt_handle.clone(),
            cache_dir,
            provider_context_cell,
            req_rx,
            res_tx,
        );

        let registry_clone = Arc::clone(&registry);
        let notify_clone = Arc::clone(&notify);
        rt_handle.spawn(async move {
            while let Some(res) = res_rx.recv().await {
                let mut reg = registry_clone.write();
                match res {
                    CacheResponse::Updated { ty, ctx, data, ts } => {
                        reg.insert(
                            (ty, ctx.hashed_key(ty)),
                            Ok(CacheEntry {
                                data,
                                updated_at: ts,
                            }),
                        );
                    }
                    CacheResponse::FetchFailed { ty, ctx, error } => {
                        reg.insert((ty, ctx.hashed_key(ty)), Err(error));
                    }
                }
                notify_clone.notify_waiters();
            }
        });

        (manager, worker)
    }

    pub fn warm(&self, ty: CacheType, ctx: CacheContext, data: AnyCacheData) {
        let key = ctx.hashed_key(ty);
        let now = time_now();

        {
            let mut reg = self.registry.write();
            reg.insert(
                (ty, key),
                Ok(CacheEntry {
                    data: data.clone(),
                    updated_at: now,
                }),
            );
        }

        self.notify.notify_waiters();

        if let Some(e) = self
            .request_tx
            .try_send(CacheCommand::Inject { ty, ctx, data })
            .err()
        {
            log::error!("Failed to send cache inject: {e}")
        }
    }

    fn trigger_fetch(&self, ty: CacheType, ctx: CacheContext, fetcher: FetchFn) {
        if let Some(e) = self
            .request_tx
            .try_send(CacheCommand::Fetch { ty, ctx, fetcher })
            .err()
        {
            log::error!("Failed to send cache request: {e}")
        }
    }

    pub fn get<T: Clone + 'static>(
        &self,
        ty: CacheType,
        ctx: CacheContext,
        fetcher: FetchFn,
    ) -> anyhow::Result<Option<T>> {
        if let Some(res) = self.get_from_registry::<T>(ty, &ctx) {
            return res.map(Some);
        }

        self.trigger_fetch(ty, ctx, fetcher);
        Ok(None)
    }

    pub async fn get_blocking<T: Clone + 'static>(
        &self,
        ty: CacheType,
        ctx: CacheContext,
        fetcher: FetchFn,
        timeout: Duration,
    ) -> anyhow::Result<Option<T>> {
        if let Some(res) = self.get_from_registry::<T>(ty, &ctx) {
            return res.map(Some);
        }

        self.trigger_fetch(ty, ctx.clone(), fetcher);

        let start = SystemTime::now();
        loop {
            if let Some(res) = self.get_from_registry::<T>(ty, &ctx) {
                return res.map(Some);
            }

            if start.elapsed().unwrap_or(timeout) >= timeout {
                return Ok(None);
            }

            let registration = self.notify.notified();
            tokio::select! {
                _ = registration => {},
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
        }
    }

    pub fn cleanup(&self) {
        let _ = self
            .request_tx
            .try_send(CacheCommand::Cleanup)
            .err()
            .map(|e| {
                log::error!("Failed to send cleanup request: {e}");
            });
    }

    pub fn clear_all(&self, tys: Vec<CacheType>, in_mem: bool) {
        if in_mem {
            let mut reg = self.registry.write();
            reg.retain(|k, _| !tys.contains(&k.0));
        }
        self.notify.notify_waiters();

        let _ = self
            .request_tx
            .try_send(CacheCommand::DeleteAll { tys })
            .err()
            .map(|e| {
                log::error!("Failed to send delete all request: {e}");
            });
    }

    pub fn discard(&self, ty: CacheType, ctx: CacheContext) {
        let key = ctx.hashed_key(ty);
        {
            let mut reg = self.registry.write();
            reg.remove(&(ty, key));
        }

        self.notify.notify_waiters();

        let _ = self
            .request_tx
            .try_send(CacheCommand::Discard { ty, ctx })
            .err()
            .map(|e| {
                log::error!("Failed to send cache discard: {e}");
            });
    }

    fn get_from_registry<T: Clone + 'static>(
        &self,
        ty: CacheType,
        ctx: &CacheContext,
    ) -> Option<anyhow::Result<T>> {
        let key = ctx.hashed_key(ty);
        let needs_discard = {
            let reg = self.registry.read();
            if let Some(entry_res) = reg.get(&(ty, key.clone())) {
                match entry_res {
                    Ok(entry) => {
                        if time_now().saturating_sub(entry.updated_at) < ty.config().ttl.as_secs() {
                            match entry.data.downcast_ref::<T>() {
                                Some(val) => return Some(Ok(val.clone())),
                                None => {
                                    log::error!(
                                        "Cache type mismatch for key: {}. Could not downcast to {}. Discarding entry.",
                                        key,
                                        std::any::type_name::<T>()
                                    );
                                    true
                                }
                            }
                        } else {
                            false
                        }
                    }
                    Err(e) => return Some(Err(anyhow::anyhow!(e.clone()))),
                }
            } else {
                false
            }
        };

        if needs_discard {
            self.discard(ty, ctx.clone());
        }

        None
    }
}
