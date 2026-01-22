use crate::app::Effect;
use crate::domain::{AppConfig, Event, ModService, ProjectType};
use crate::infra::{ApiService, ConfigManager, IconService, IconWorker, LegacyListService};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AppRuntime {
    pub mod_service: Arc<ModService>,
    pub config_manager: Arc<ConfigManager>,
    pub icon_service: IconService,

    rt_handle: tokio::runtime::Handle,
    event_tx: mpsc::Sender<Event>,

    api_service: Arc<ApiService>,
    legacy_service: Arc<LegacyListService>,
}

impl AppRuntime {
    pub fn new(rt_handle: tokio::runtime::Handle) -> (Self, mpsc::Receiver<Event>) {
        let (event_tx, event_rx) = mpsc::channel::<Event>(200);

        let api_service = Arc::new(ApiService::new());
        let config_manager =
            Arc::new(ConfigManager::new().expect("Failed to create config manager"));
        let mod_service = Arc::new(ModService::new(
            api_service.clone(),
            config_manager.get_cache_dir().to_path_buf(),
        ));
        let legacy_service = Arc::new(LegacyListService::new(mod_service.clone()));

        let (icon_tx, icon_rx) = mpsc::channel::<(String, Vec<u8>)>(100);
        let (icon_url_tx, icon_url_rx) = mpsc::channel::<String>(100);

        let icon_worker = IconWorker::new(
            api_service.clone(),
            config_manager.get_cache_dir().to_path_buf(),
            icon_url_rx,
            icon_tx,
        );
        rt_handle.spawn(icon_worker.run());

        let icon_service = IconService::new(icon_rx, icon_url_tx);

        let disk_cache = mod_service.get_disk_cache();
        rt_handle.spawn(async move {
            loop {
                disk_cache.clear_expired().await;
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });

        (
            Self {
                mod_service,
                config_manager,
                icon_service,
                rt_handle,
                event_tx,
                api_service,
                legacy_service,
            },
            event_rx,
        )
    }

    pub fn get_project_link(&self, project_type: &ProjectType, mod_id: &str) -> String {
        self.api_service
            .provider
            .get_project_link(project_type, mod_id)
    }

    pub fn enqueue(&self, effect: Effect) {
        self.run_effect(effect);
    }

    pub fn enqueue_all(&self, effects: Vec<Effect>) {
        for e in effects {
            self.enqueue(e);
        }
    }

    fn run_effect(&self, effect: Effect) {
        match effect {
            Effect::LoadInitialData => {
                let cm = self.config_manager.clone();
                let prov = self.api_service.provider.clone();
                let tx = self.event_tx.clone();

                self.rt_handle.spawn(async move {
                    let _ = cm.ensure_dirs().await;

                    let config = if cm.config_exists() {
                        cm.load_config().await.unwrap_or_else(|_| AppConfig {
                            current_list_id: None,
                            default_list_name: "New List".to_string(),
                            selected_version: String::new(),
                            selected_loader: String::new(),
                            download_dir: String::new(),
                        })
                    } else {
                        cm.create_default_config().await.unwrap_or(AppConfig {
                            current_list_id: None,
                            default_list_name: "New List".to_string(),
                            selected_version: String::new(),
                            selected_loader: String::new(),
                            download_dir: String::new(),
                        })
                    };

                    let lists = cm.load_all_lists().await.unwrap_or_default();

                    let current_list_id = config
                        .current_list_id
                        .clone()
                        .filter(|id| lists.iter().any(|l| &l.id == id));

                    let versions = prov.get_minecraft_versions().await.unwrap_or_else(|_| {
                        vec![crate::domain::MinecraftVersion {
                            id: "1.20.1".to_string(),
                            name: "1.20.1".to_string(),
                        }]
                    });

                    let loaders = prov
                        .get_mod_loaders_for_type(ProjectType::Mod)
                        .await
                        .unwrap_or_default();

                    let _ = tx
                        .send(Event::InitialDataLoaded {
                            mod_lists: lists,
                            current_list_id,
                            minecraft_versions: versions,
                            mod_loaders: loaders,
                            default_list_name: config.default_list_name,
                        })
                        .await;
                });
            }

            Effect::LoadLoadersForType { project_type } => {
                let api_svc = self.api_service.clone();
                let tx = self.event_tx.clone();

                self.rt_handle.spawn(async move {
                    let loaders = api_svc
                        .provider
                        .get_mod_loaders_for_type(project_type)
                        .await
                        .unwrap_or_default();

                    let _ = tx
                        .send(Event::LoadersForTypeLoaded {
                            project_type,
                            loaders,
                        })
                        .await;
                });
            }

            Effect::SearchMods {
                query,
                version,
                loader,
                project_type,
            } => {
                let api_svc = self.api_service.clone();
                let mod_svc = self.mod_service.clone();
                let tx = self.event_tx.clone();
                let ver_clone = version.clone();
                let loader_clone = loader.clone();

                self.rt_handle.spawn(async move {
                    let _permit = api_svc.limiter.acquire(1).await;

                    match api_svc
                        .provider
                        .search_mods(&query, &version, &loader, &project_type)
                        .await
                    {
                        Ok(results) => {
                            let cached = mod_svc
                                .cache_search_results(results, ver_clone, loader_clone)
                                .await;
                            let _ = tx.send(Event::SearchResults(cached)).await;
                        }
                        Err(_) => {
                            log::warn!("Failed to search: {query}");
                        }
                    }
                });
            }

            Effect::FetchModDetails {
                mod_id,
                version,
                loader,
            } => {
                let mod_svc = self.mod_service.clone();
                let tx = self.event_tx.clone();
                let version_clone = version.clone();
                let loader_clone = loader.clone();

                self.rt_handle.spawn(async move {
                    match mod_svc.get_mod_by_id(&mod_id, &version, &loader).await {
                        Ok(info) => {
                            let _ = tx
                                .send(Event::ModDetails {
                                    info,
                                    version: version_clone,
                                    loader: loader_clone,
                                })
                                .await;
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch details for {mod_id}: {e}");
                            let _ = tx.send(Event::ModDetailsFailed { mod_id }).await;
                        }
                    }
                });
            }

            Effect::DownloadMod {
                mod_info,
                download_dir,
            } => {
                let api_svc = self.api_service.clone();
                let tx = self.event_tx.clone();

                self.rt_handle.spawn(async move {
                    let _permit = api_svc.limiter.acquire(3).await;

                    let mod_id = mod_info.id.clone();
                    let mod_version = mod_info.version.clone();
                    let filename = crate::domain::generate_mod_filename(&mod_info);
                    let destination = std::path::Path::new(&download_dir).join(&filename);

                    let tx_progress = tx.clone();
                    let mod_id_clone = mod_id.clone();

                    let result = api_svc
                        .provider
                        .download_mod(
                            &mod_info.download_url,
                            &destination,
                            Box::new(move |progress| {
                                let _ = tx_progress.try_send(Event::DownloadProgress {
                                    mod_id: mod_id_clone.clone(),
                                    progress,
                                });
                            }),
                        )
                        .await;

                    let success = result.is_ok();

                    if success {
                        let download_path = std::path::Path::new(&download_dir);
                        if let Err(e) = crate::infra::update_metadata_entry(
                            download_path,
                            mod_id.clone(),
                            filename,
                            mod_version,
                        )
                        .await
                        {
                            log::warn!("Failed to update download metadata: {e}");
                        }
                    }

                    let _ = tx.send(Event::DownloadComplete { mod_id, success }).await;
                });
            }

            Effect::SaveList { list } => {
                let cm = self.config_manager.clone();
                self.rt_handle.spawn(async move {
                    let _ = cm.save_list(&list).await;
                });
            }

            Effect::DeleteList { list_id } => {
                let cm = self.config_manager.clone();
                self.rt_handle.spawn(async move {
                    let _ = cm.delete_list(&list_id).await;
                });
            }

            Effect::SaveConfig {
                current_list_id,
                default_list_name,
            } => {
                let cm = self.config_manager.clone();
                let config = AppConfig {
                    current_list_id,
                    default_list_name,
                    selected_version: String::new(),
                    selected_loader: String::new(),
                    download_dir: String::new(),
                };
                self.rt_handle.spawn(async move {
                    let _ = cm.save_config(&config).await;
                });
            }

            Effect::ExportListToml { path, list } => {
                self.rt_handle.spawn(async move {
                    let toml_string = toml::to_string_pretty(&list).unwrap_or_default();
                    let _ = tokio::fs::write(path, toml_string).await;
                });
            }

            Effect::LegacyListImport {
                path,
                version,
                loader,
            } => {
                let legacy_svc = self.legacy_service.clone();
                let tx = self.event_tx.clone();
                self.rt_handle.spawn(async move {
                    legacy_svc
                        .import_legacy_list(path, version, loader, tx)
                        .await;
                });
            }

            Effect::LegacyListExport {
                path,
                mod_ids,
                version,
                loader,
            } => {
                let legacy_svc = self.legacy_service.clone();
                let tx = self.event_tx.clone();
                self.rt_handle.spawn(async move {
                    legacy_svc
                        .export_legacy_list(path, mod_ids, version, loader, tx)
                        .await;
                });
            }

            Effect::RemoveFromMetadata {
                download_dir,
                mod_id,
            } => {
                self.rt_handle.spawn(async move {
                    let download_path = std::path::Path::new(&download_dir);
                    if let Err(e) =
                        crate::infra::remove_metadata_entry(download_path, &mod_id).await
                    {
                        log::warn!("Failed to remove metadata entry for {mod_id}: {e}");
                    }
                });
            }

            Effect::DeleteModFile {
                download_dir,
                mod_id,
            } => {
                self.rt_handle.spawn(async move {
                    let download_path = std::path::Path::new(&download_dir);

                    if let Ok(metadata) = crate::infra::read_download_metadata(download_path).await
                        && let Some(entry) = metadata.get_entry(&mod_id)
                    {
                        let file_path = download_path.join(&entry.file);
                        if file_path.exists() {
                            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                                log::warn!("Failed to delete mod file {}: {}", entry.file, e);
                            } else {
                                log::info!("Deleted mod file: {}", entry.file);
                            }
                        }

                        let archived_path = download_path.join(format!("{}.archived", entry.file));
                        if archived_path.exists() {
                            if let Err(e) = tokio::fs::remove_file(&archived_path).await {
                                log::warn!(
                                    "Failed to delete archived file {}.archived: {}",
                                    entry.file,
                                    e
                                );
                            } else {
                                log::info!("Deleted archived file: {}.archived", entry.file);
                            }
                        }
                    }
                });
            }

            Effect::DeleteUnknownFile {
                download_dir,
                filename,
            } => {
                self.rt_handle.spawn(async move {
                    let download_path = std::path::Path::new(&download_dir);
                    let file_path = download_path.join(&filename);

                    if file_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(&file_path).await {
                            log::warn!("Failed to delete unknown file {filename}: {e}");
                        } else {
                            log::info!("Deleted unknown file: {filename}");
                        }
                    }
                });
            }

            Effect::ArchiveModFile {
                download_dir,
                mod_id,
            } => {
                let tx = self.event_tx.clone();
                let dir_clone = download_dir.clone();

                self.rt_handle.spawn(async move {
                    let download_path = std::path::Path::new(&download_dir);

                    if let Ok(mut metadata) =
                        crate::infra::read_download_metadata(download_path).await
                        && let Some(entry) = metadata.get_entry(&mod_id)
                    {
                        let original_file = entry.file.clone();
                        let version = entry.version.clone();
                        let file_path = download_path.join(&original_file);
                        let archived_path = download_path.join(format!("{original_file}.archived"));

                        if file_path.exists() {
                            if let Err(e) = tokio::fs::rename(&file_path, &archived_path).await {
                                log::warn!("Failed to archive mod file {original_file}: {e}");
                            } else {
                                log::info!(
                                    "Archived mod file: {original_file} -> {original_file}.archived"
                                );

                                metadata.update_entry(
                                    mod_id.clone(),
                                    format!("{original_file}.archived"),
                                    version,
                                );

                                if let Err(e) =
                                    crate::infra::write_download_metadata(download_path, &metadata)
                                        .await
                                {
                                    log::warn!("Failed to update metadata after archiving: {e}");
                                } else {
                                    let _ = tx
                                        .send(Event::MetadataLoaded {
                                            download_dir: dir_clone,
                                            metadata,
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                });
            }

            Effect::UnarchiveModFile {
                download_dir,
                mod_id,
            } => {
                let tx = self.event_tx.clone();
                let dir_clone = download_dir.clone();

                self.rt_handle.spawn(async move {
                    let download_path = std::path::Path::new(&download_dir);

                    if let Ok(mut metadata) =
                        crate::infra::read_download_metadata(download_path).await
                        && let Some(entry) = metadata.get_entry(&mod_id)
                    {
                        let archived_file = entry.file.clone();
                        let version = entry.version.clone();

                        if let Some(original_file) = archived_file.strip_suffix(".archived") {
                            let archived_path = download_path.join(&archived_file);
                            let file_path = download_path.join(original_file);

                            if archived_path.exists() {
                                if let Err(e) = tokio::fs::rename(&archived_path, &file_path).await
                                {
                                    log::warn!("Failed to unarchive mod file {archived_file}: {e}");
                                } else {
                                    log::info!(
                                        "Unarchived mod file: {archived_file} -> {original_file}"
                                    );

                                    metadata.update_entry(
                                        mod_id.clone(),
                                        original_file.to_string(),
                                        version,
                                    );

                                    if let Err(e) = crate::infra::write_download_metadata(
                                        download_path,
                                        &metadata,
                                    )
                                    .await
                                    {
                                        log::warn!(
                                            "Failed to update metadata after unarchiving: {e}"
                                        );
                                    } else {
                                        let _ = tx
                                            .send(Event::MetadataLoaded {
                                                download_dir: dir_clone,
                                                metadata,
                                            })
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                });
            }

            Effect::ValidateMetadata { download_dir } => {
                let tx = self.event_tx.clone();
                let dir_clone = download_dir.clone();
                self.rt_handle.spawn(async move {
                    let download_path = std::path::Path::new(&download_dir);
                    match crate::infra::read_download_metadata(download_path).await {
                        Ok(mut metadata) => {
                            metadata.validate_and_cleanup(download_path);
                            if let Err(e) =
                                crate::infra::write_download_metadata(download_path, &metadata)
                                    .await
                            {
                                log::warn!("Failed to write validated metadata: {e}");
                            } else {
                                let _ = tx
                                    .send(Event::MetadataLoaded {
                                        download_dir: dir_clone,
                                        metadata,
                                    })
                                    .await;
                            }
                        }
                        Err(e) => {
                            log::debug!("Could not read metadata for validation: {e}");
                            let _ = tx
                                .send(Event::MetadataLoaded {
                                    download_dir: dir_clone,
                                    metadata: crate::infra::DownloadMetadata::new(),
                                })
                                .await;
                        }
                    }
                });
            }
        }
    }
}
