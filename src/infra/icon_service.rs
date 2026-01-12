use egui::TextureHandle;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

pub struct IconService {
    handles: HashMap<String, TextureHandle>,
    loading: HashSet<String>,
    rx: mpsc::Receiver<(String, Vec<u8>)>,
    url_tx: mpsc::Sender<String>,
}

impl IconService {
    pub fn new(rx: mpsc::Receiver<(String, Vec<u8>)>, url_tx: mpsc::Sender<String>) -> Self {
        Self {
            handles: HashMap::new(),
            loading: HashSet::new(),
            rx,
            url_tx,
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
                    self.handles.insert(url.clone(), texture_handle);
                    self.loading.remove(&url);
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

        let _ = self.url_tx.try_send(url.to_string());

        None
    }
}
