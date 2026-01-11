use crate::infra::ApiService;
use egui::TextureHandle;
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct IconService {
    handles: HashMap<String, TextureHandle>,
    loading: HashSet<String>,
    rx: mpsc::Receiver<(String, Vec<u8>)>,
    tx: mpsc::Sender<(String, Vec<u8>)>,
    api_service: Arc<ApiService>,
    cache_dir: PathBuf,
    runtime_handle: tokio::runtime::Handle,
}

impl IconService {
    pub fn new(
        api_service: Arc<ApiService>,
        config_dir: PathBuf,
        runtime_handle: tokio::runtime::Handle,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            handles: HashMap::new(),
            loading: HashSet::new(),
            rx,
            tx,
            api_service,
            cache_dir: config_dir,
            runtime_handle,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        while let Ok((url, bytes)) = self.rx.try_recv() {
            match image::load_from_memory(&bytes) {
                Ok(img) => {
                    let size = [img.width() as _, img.height() as _];
                    let rgba = img.to_rgba8();
                    let texture_handle = ctx.load_texture(
                        &url,
                        egui::ColorImage::from_rgba_unmultiplied(size, &rgba),
                        Default::default(),
                    );
                    self.handles.insert(url, texture_handle);
                }
                Err(e) => {
                    log::error!("Failed to decode image for {}: {}", url, e);
                    self.loading.remove(&url);
                }
            }
        }
    }

    pub fn get(&mut self, url: &str) -> Option<&TextureHandle> {
        if let Some(handle) = self.handles.get(url) {
            return Some(handle);
        }

        if self.loading.contains(url) {
            return None;
        }

        self.loading.insert(url.to_string());
        self.start_download(url.to_string());

        None
    }

    fn start_download(&self, url: String) {
        let tx = self.tx.clone();
        let api_service = self.api_service.clone();
        let cache_dir = self.cache_dir.clone();
        let runtime_handle = self.runtime_handle.clone();

        runtime_handle.spawn(async move {
            let mut hasher = DefaultHasher::new();
            url.hash(&mut hasher);
            let hash = format!("{:x}", hasher.finish());
            let icon_path = cache_dir.join("icons").join(&hash);

            let bytes = if icon_path.exists() {
                tokio::fs::read(&icon_path).await.ok()
            } else {
                let _permit = api_service.limiter.acquire(1).await;

                match reqwest::get(&url).await {
                    Ok(response) => match response.bytes().await {
                        Ok(data) => {
                            if let Some(parent) = icon_path.parent() {
                                let _ = tokio::fs::create_dir_all(parent).await;
                            }
                            let _ = tokio::fs::write(&icon_path, &data).await;
                            Some(data.to_vec())
                        }
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            };

            if let Some(data) = bytes {
                let _ = tx.send((url, data)).await;
            }
        });
    }
}
