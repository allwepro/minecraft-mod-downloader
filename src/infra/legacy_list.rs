use crate::domain::{ConnectionLimiter, Event, ModProvider};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

#[derive(Default)]
pub struct SlugCache {
    slug_to_id: HashMap<String, String>,
    id_to_slug: HashMap<String, String>,
}

impl SlugCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_id(&self, slug: &str) -> Option<&str> {
        self.slug_to_id.get(slug).map(|s| s.as_str())
    }

    pub fn get_slug(&self, id: &str) -> Option<&str> {
        self.id_to_slug.get(id).map(|s| s.as_str())
    }

    pub fn insert(&mut self, slug: String, id: String) {
        self.slug_to_id.insert(slug.clone(), id.clone());
        self.id_to_slug.insert(id, slug);
    }
}

pub struct LegacyListService {
    pub provider: Arc<dyn ModProvider>,
    pub limiter: Arc<ConnectionLimiter>,
}

impl LegacyListService {
    pub fn new(provider: Arc<dyn ModProvider>, limiter: Arc<ConnectionLimiter>) -> Self {
        Self { provider, limiter }
    }

    pub async fn import_legacy_list(
        &self,
        path: PathBuf,
        version: String,
        loader: String,
        tx: mpsc::Sender<Event>,
    ) {
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                let _ = tx
                    .send(Event::LegacyListFailed {
                        error: format!("Failed to read file: {}", e),
                        is_import: true,
                    })
                    .await;
                return;
            }
        };

        let slugs: Vec<String> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect();

        let mut successful_mods = Vec::new();
        let mut failed = Vec::new();
        let mut warnings = Vec::new();

        for (idx, slug) in slugs.iter().enumerate() {
            let _ = tx
                .send(Event::LegacyListProgress {
                    current: idx + 1,
                    total: slugs.len(),
                    message: format!("Resolving '{}'...", slug),
                })
                .await;

            let _permit = self.limiter.acquire(1).await;
            match self
                .provider
                .fetch_mod_details(slug, &version, &loader)
                .await
            {
                Ok(mod_info) => successful_mods.push(mod_info),
                Err(e) => {
                    log::warn!("Failed to resolve slug '{}': {}", slug, e);
                    failed.push(slug.clone());
                }
            }
        }

        let _ = tx
            .send(Event::LegacyListComplete {
                successful: successful_mods,
                failed,
                warnings,
                is_import: true,
            })
            .await;
    }

    pub async fn export_legacy_list(
        &self,
        path: PathBuf,
        mod_ids: Vec<String>,
        version: String,
        loader: String,
        tx: mpsc::Sender<Event>,
    ) {
        let mut successful_mods = Vec::new();
        let mut failed = Vec::new();
        let mut warnings = Vec::new();
        let mut slugs = Vec::new();

        for (idx, mod_id) in mod_ids.iter().enumerate() {
            let _ = tx
                .send(Event::LegacyListProgress {
                    current: idx + 1,
                    total: mod_ids.len(),
                    message: format!("Resolving '{}'...", mod_id),
                })
                .await;

            let _permit = self.limiter.acquire(1).await;
            match self
                .provider
                .fetch_mod_details(mod_id, &version, &loader)
                .await
            {
                Ok(mod_info) => {
                    if mod_info.slug.is_empty() {
                        warnings.push(format!("Mod '{}' has no slug, skipping", mod_id));
                        failed.push(mod_id.clone());
                    } else {
                        slugs.push(mod_info.slug.clone());
                        successful_mods.push(mod_info);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to resolve ID '{}': {}", mod_id, e);
                    failed.push(mod_id.clone());
                }
            }
        }

        let temp_path = path.with_extension("mods.tmp");
        let content = format!(
            "# Minecraft Mod List\n# Generated on {}\n\n{}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            slugs.join("\n")
        );

        if let Err(e) = tokio::fs::write(&temp_path, content).await {
            let _ = tx
                .send(Event::LegacyListFailed {
                    error: format!("Failed to write file: {}", e),
                    is_import: false,
                })
                .await;
            return;
        }

        if let Err(e) = tokio::fs::rename(temp_path, &path).await {
            let _ = tx
                .send(Event::LegacyListFailed {
                    error: format!("Failed to finalize file: {}", e),
                    is_import: false,
                })
                .await;
            return;
        }

        let _ = tx
            .send(Event::LegacyListComplete {
                successful: successful_mods,
                failed,
                warnings,
                is_import: false,
            })
            .await;
    }
}
