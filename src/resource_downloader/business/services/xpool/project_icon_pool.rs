use crate::resource_downloader::business::xcache::{
    AnyCacheData, CacheContext, CacheType, CoreCacheManager,
};
use crate::resource_downloader::domain::ProjectLnk;
use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::{ImageBuffer, Rgba, RgbaImage};
use parking_lot::RwLock;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::sync::Arc;

pub struct ProjectIconPool {
    cache: Arc<CoreCacheManager>,
    /// Stores the actual GPU texture handles.
    textures: Arc<RwLock<HashMap<ProjectLnk, TextureHandle>>>,
    loading: Arc<RwLock<HashSet<ProjectLnk>>>,
}

impl ProjectIconPool {
    pub fn new(cache: Arc<CoreCacheManager>) -> Self {
        Self {
            cache,
            textures: Arc::new(RwLock::new(HashMap::new())),
            loading: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Returns the TextureHandle if already loaded, otherwise triggers an async fetch and returns None.
    pub fn get_icon(&self, project: &ProjectLnk, name: &str) -> Option<TextureHandle> {
        if let Some(handle) = self.textures.read().get(project) {
            return Some(handle.clone());
        }

        let project_clone = project.clone();
        let pool_self = self.clone_handle();

        pool_self.internal_fetch(project_clone, name.to_string());

        None
    }

    /// Updates the pool by checking for completed fetches and uploading textures to GPU. Has to be in the egui update loop.
    pub fn update(&self, ctx: &Context) {
        let loading_list: Vec<ProjectLnk> = self.loading.read().iter().cloned().collect();

        for project in loading_list {
            let cache_ctx = CacheContext {
                id: Some(project.to_context_id()),
                resource_type: None,
                version: None,
                loader: None,
            };
            let cache_key = cache_ctx.hashed_key(CacheType::ProjectIcons);

            let res = self.cache.get::<Vec<u8>>(
                CacheType::ProjectIcons,
                cache_ctx,
                Box::new(|_| Box::pin(async { Err(anyhow::anyhow!("Polling should not fetch")) })),
            );

            match res {
                Ok(Some(bytes)) => match self.decode_and_upload(ctx, &project, &bytes) {
                    Ok(handle) => {
                        let mut textures = self.textures.write();
                        let mut loading = self.loading.write();
                        textures.insert(project.clone(), handle);
                        loading.remove(&project);
                    }
                    Err(e) => {
                        log::error!("Failed to decode icon for {project} ({cache_key}): {e}");
                        self.loading.write().remove(&project);
                    }
                },
                Err(e) => {
                    log::error!("Icon fetch failed for {project} ({cache_key}): {e}");
                    self.loading.write().remove(&project);
                }
                Ok(None) => {}
            }
        }
    }

    /// Clears all cached textures and loading states.
    #[allow(dead_code)]
    pub fn clear_gpu_cache(&self) {
        self.textures.write().clear();
        self.loading.write().clear();
    }

    fn internal_fetch(&self, project: ProjectLnk, name: String) {
        let mut loading = self.loading.write();
        if self.textures.read().contains_key(&project) || !loading.insert(project.clone()) {
            return;
        }

        let cache_ctx = CacheContext {
            id: Some(project.to_context_id()),
            resource_type: None,
            version: None,
            loader: None,
        };

        let name_clone = name.clone();
        let _ = self.cache.get::<Vec<u8>>(
            CacheType::ProjectIcons,
            cache_ctx,
            Box::new(move |p_ctx| {
                let project_clone = project.clone();
                let name = name_clone.clone();
                Box::pin(async move {
                    match p_ctx
                        .provider
                        .load_project_icon(&p_ctx, project_clone)
                        .await
                    {
                        Ok(bytes) => Ok(Arc::new(bytes.to_vec()) as AnyCacheData),
                        Err(_) => {
                            let fallback = generate_procedural_icon(&name);
                            Ok(Arc::new(fallback) as AnyCacheData)
                        }
                    }
                })
            }),
        );
    }

    fn decode_and_upload(
        &self,
        ctx: &Context,
        lnk: &ProjectLnk,
        bytes: &[u8],
    ) -> anyhow::Result<TextureHandle> {
        let image = image::load_from_memory(bytes)?;
        let size = [image.width() as usize, image.height() as usize];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        Ok(ctx.load_texture(lnk.to_string(), color_image, TextureOptions::default()))
    }

    fn clone_handle(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            textures: self.textures.clone(),
            loading: self.loading.clone(),
        }
    }
}

fn generate_procedural_icon(name: &str) -> Vec<u8> {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();

    let size = 128;
    let mut img: RgbaImage = ImageBuffer::new(size, size);

    let r1 = (hash & 0xFF) as u8;
    let g1 = ((hash >> 8) & 0xFF) as u8;
    let b1 = ((hash >> 16) & 0xFF) as u8;

    let r2 = ((hash >> 24) & 0xFF) as u8;
    let g2 = ((hash >> 32) & 0xFF) as u8;
    let b2 = ((hash >> 40) & 0xFF) as u8;

    let bg = Rgba([(r1 % 60) + 30, (g1 % 60) + 30, (b1 % 60) + 30, 255]);
    let fg = Rgba([r2.max(160), g2.max(160), b2.max(160), 255]);

    for p in img.pixels_mut() {
        *p = bg;
    }

    let center = size as f32 / 2.0;
    let shape_size = size as f32 * 0.3;
    let shape_type = hash % 4;

    for y in 0..size {
        for x in 0..size {
            let mut hits = 0;
            for sy in 0..2 {
                for sx in 0..2 {
                    let px = x as f32 + (sx as f32 + 0.5) / 2.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 2.0;

                    let dx = px - center;
                    let dy = py - center;

                    let inside = match shape_type {
                        0 => (dx * dx + dy * dy) < shape_size * shape_size,
                        1 => dx.abs() < shape_size && dy.abs() < shape_size,
                        2 => {
                            let h = shape_size * 1.5;
                            dy < h / 3.0 && dy > -2.0 * h / 3.0 && dx.abs() < (h / 3.0 - dy) * 0.8
                        }
                        _ => dx.abs() + dy.abs() < shape_size * 1.3,
                    };

                    if inside {
                        hits += 1;
                    }
                }
            }

            if hits > 0 {
                let alpha = hits as f32 / 4.0;
                let current = img.get_pixel(x, y);
                let blended = Rgba([
                    (fg[0] as f32 * alpha + current[0] as f32 * (1.0 - alpha)) as u8,
                    (fg[1] as f32 * alpha + current[1] as f32 * (1.0 - alpha)) as u8,
                    (fg[2] as f32 * alpha + current[2] as f32 * (1.0 - alpha)) as u8,
                    255,
                ]);
                img.put_pixel(x, y, blended);
            }
        }
    }

    let mut bytes = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    bytes
}
