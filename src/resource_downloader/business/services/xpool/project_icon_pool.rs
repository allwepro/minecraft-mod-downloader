use crate::resource_downloader::business::xcache::{
    AnyCacheData, CacheContext, CacheType, CoreCacheManager,
};
use crate::resource_downloader::domain::ProjectLnk;
use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
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
    pub fn get_icon(&self, project: &ProjectLnk) -> Option<TextureHandle> {
        if let Some(handle) = self.textures.read().get(project) {
            return Some(handle.clone());
        }

        let project_clone = project.clone();
        let pool_self = self.clone_handle();

        pool_self.internal_fetch(project_clone);

        None
    }

    /// Updates the pool by checking for completed fetches and uploading textures to GPU. Has to be in the egui update loop.
    pub fn update(&self, ctx: &Context) {
        let loading_list: Vec<ProjectLnk> = self.loading.read().iter().cloned().collect();

        for project in loading_list {
            let cache_ctx = CacheContext {
                id: project.to_context_id(),
                resource_type: None,
                version: None,
                loader: None,
            };

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
                        log::error!("Failed to decode icon for {project}: {e}");
                        self.loading.write().remove(&project);
                    }
                },
                Err(e) => {
                    log::error!("Icon fetch failed for {project}: {e}");
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

    fn internal_fetch(&self, project: ProjectLnk) {
        let mut loading = self.loading.write();
        if self.textures.read().contains_key(&project) || !loading.insert(project.clone()) {
            return;
        }

        let cache_ctx = CacheContext {
            id: project.to_context_id(),
            resource_type: None,
            version: None,
            loader: None,
        };

        let _ = self.cache.get::<Vec<u8>>(
            CacheType::ProjectIcons,
            cache_ctx,
            Box::new(move |p_ctx| {
                Box::pin(async move {
                    let bytes = p_ctx.provider.load_project_icon(&p_ctx, project).await?;
                    Ok(Arc::new(bytes.to_vec()) as AnyCacheData)
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
