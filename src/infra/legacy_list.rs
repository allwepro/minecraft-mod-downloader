use crate::domain::{Event, ModInfo, ModService};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct LegacyListService {
    mod_service: Arc<ModService>,
}

impl LegacyListService {
    pub fn new(mod_service: Arc<ModService>) -> Self {
        Self { mod_service }
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

            match self
                .mod_service
                .get_mod_by_slug(slug, &version, &loader)
                .await
            {
                Ok(info) => {
                    successful_mods.push(info);
                }
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

            match self
                .mod_service
                .get_mod_by_id(mod_id, &version, &loader)
                .await
            {
                Ok(mod_info) => {
                    let info_ref = mod_info.as_ref();
                    if info_ref.slug.is_empty() {
                        warnings.push(format!("Mod '{}' has no slug, skipping", mod_id));
                        failed.push(mod_id.clone());
                    } else {
                        slugs.push(info_ref.slug.clone());
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
