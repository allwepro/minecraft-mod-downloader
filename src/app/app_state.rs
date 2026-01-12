use crate::app::*;
use crate::domain::*;
use crate::domain::{AppConfig, ModList, ModService};
use crate::infra::LegacyListService;
use crate::infra::*;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AppState {
    pub mod_service: Arc<ModService>,
    pub config_manager: Arc<ConfigManager>,
    pub minecraft_versions: Vec<MinecraftVersion>,
    pub mod_loaders: Vec<ModLoader>,
    pub selected_version: String,
    pub selected_loader: String,
    pub(crate) previous_version: String,
    pub(crate) previous_loader: String,
    pub mod_lists: Vec<ModList>,
    pub current_list_id: Option<String>,
    pub download_progress: HashMap<String, f32>,
    pub download_status: HashMap<String, DownloadStatus>,
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
    pub search_window_results: Vec<Arc<ModInfo>>,
    pub mods_being_loaded: HashSet<String>,
    pub mods_failed_loading: HashSet<String>,
    pub download_dir: String,
    _runtime: tokio::runtime::Runtime,
    runtime_handle: tokio::runtime::Handle,
    pub legacy_state: LegacyState,
    pub pending_legacy_mods: Option<Vec<Arc<ModInfo>>>,
    pub icon_service: IconService,
    pub search_filter_exact: bool,
}

impl AppState {
    pub fn new(runtime: tokio::runtime::Runtime) -> Self {
        let runtime_handle = runtime.handle().clone();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(100);
        let (event_tx, event_rx) = mpsc::channel::<Event>(100);

        let api_service = Arc::new(ApiService::new());
        let api_service_for_spawn = api_service.clone();

        let mod_service = Arc::new(ModService::new(api_service.clone()));
        let mod_service_for_spawn = mod_service.clone();

        let legacy_service = Arc::new(LegacyListService::new(mod_service_for_spawn.clone()));
        let legacy_service_for_spawn = legacy_service.clone();

        runtime_handle.spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let event_tx = event_tx.clone();
                let api_svc = api_service_for_spawn.clone();
                let legacy_svc = legacy_service_for_spawn.clone();
                let mod_svc = mod_service_for_spawn.clone();

                match cmd {
                    Command::SearchMods {
                        query,
                        version,
                        loader,
                    } => {
                        tokio::spawn(async move {
                            let _permit = api_svc.limiter.acquire(1).await;

                            if let Ok(results) = api_svc
                                .provider
                                .search_mods(&query, &version, &loader)
                                .await
                            {
                                let cached_results = mod_svc.cache_search_results(results).await;
                                let _ = event_tx.send(Event::SearchResults(cached_results)).await;
                            } else {
                                log::warn!("Failed to search mods: {}", query);
                            }
                        });
                    }
                    Command::FetchModDetails {
                        mod_id,
                        version,
                        loader,
                    } => {
                        tokio::spawn(async move {
                            match mod_svc.get_mod_by_id(&mod_id, &version, &loader).await {
                                Ok(info) => {
                                    let _ = event_tx.send(Event::ModDetails(info)).await;
                                }
                                Err(e) => {
                                    log::warn!("Failed to fetch mod details for {}: {}", mod_id, e);
                                    let _ = event_tx.send(Event::ModDetailsFailed { mod_id }).await;
                                }
                            }
                        });
                    }
                    Command::DownloadMod {
                        mod_info,
                        download_dir,
                    } => {
                        tokio::spawn(async move {
                            let _permit = api_svc.limiter.acquire(3).await;

                            let mod_id = mod_info.id.clone();
                            let filename = format!("{}.jar", mod_info.name.replace(" ", "_"));
                            let destination = std::path::Path::new(&download_dir).join(&filename);

                            let tx_progress = event_tx.clone();
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

                            let _ = event_tx
                                .send(Event::DownloadComplete {
                                    mod_id,
                                    success: result.is_ok(),
                                })
                                .await;
                        });
                    }
                    Command::LegacyListImport {
                        path,
                        version,
                        loader,
                    } => {
                        tokio::spawn(async move {
                            legacy_svc
                                .import_legacy_list(path, version, loader, event_tx)
                                .await;
                        });
                    }
                    Command::LegacyListExport {
                        path,
                        mod_ids,
                        version,
                        loader,
                    } => {
                        tokio::spawn(async move {
                            legacy_svc
                                .export_legacy_list(path, mod_ids, version, loader, event_tx)
                                .await;
                        });
                    }
                }
            }
        });

        let config_manager =
            Arc::new(ConfigManager::new().expect("Failed to create config manager"));

        let icon_service = IconService::new(
            api_service.clone(),
            config_manager.get_cache_dir().to_path_buf(),
            runtime_handle.clone(),
        );

        let (
            selected_version,
            selected_loader,
            download_dir,
            mod_lists,
            current_list_id,
            minecraft_versions,
            mod_loaders,
        ) = {
            let cm = config_manager.clone();
            let prov = api_service.provider.clone();

            runtime_handle.block_on(async {
                let _ = cm.ensure_dirs().await;

                let config = if cm.config_exists() {
                    cm.load_config().await.unwrap_or_else(|_| {
                        runtime_handle.block_on(cm.create_default_config()).unwrap()
                    })
                } else {
                    cm.create_default_config().await.unwrap()
                };

                let lists = cm.load_all_lists().await.unwrap_or_else(|_| Vec::new());

                let current_list_id = config
                    .current_list_id
                    .filter(|id| lists.iter().any(|l| &l.id == id));

                let versions = prov.get_minecraft_versions().await.unwrap_or_else(|_| {
                    vec![MinecraftVersion {
                        id: "1.20.1".to_string(),
                        name: "1.20.1".to_string(),
                    }]
                });

                let loaders = prov.get_mod_loaders().await.unwrap_or_else(|_| {
                    vec![ModLoader {
                        id: "fabric".to_string(),
                        name: "Fabric".to_string(),
                    }]
                });

                (
                    config.selected_version,
                    config.selected_loader,
                    config.download_dir,
                    lists,
                    current_list_id,
                    versions,
                    loaders,
                )
            })
        };

        Self {
            mod_service,
            config_manager,
            minecraft_versions,
            mod_loaders,
            previous_version: selected_version.clone(),
            previous_loader: selected_loader.clone(),
            selected_version,
            selected_loader,
            mod_lists,
            current_list_id,
            download_progress: HashMap::new(),
            download_status: HashMap::new(),
            cmd_tx,
            event_rx,
            search_window_results: Vec::new(),
            mods_being_loaded: HashSet::new(),
            mods_failed_loading: HashSet::new(),
            download_dir,
            _runtime: runtime,
            runtime_handle,
            legacy_state: LegacyState::Idle,
            pending_legacy_mods: None,
            icon_service,
            search_filter_exact: true,
        }
    }

    pub fn is_mod_file_present(&self, mod_info: &ModInfo) -> bool {
        let download_dir = self.get_effective_download_dir();
        if download_dir.is_empty() {
            return false;
        }
        let filename = format!("{}.jar", mod_info.name.replace(" ", "_"));
        let path = std::path::Path::new(&download_dir).join(filename);
        path.exists()
    }

    fn get_current_list_settings(&self) -> (Option<String>, Option<String>, Option<String>) {
        if let Some(list) = self.get_current_list() {
            let ver = if list.version.is_empty() {
                None
            } else {
                Some(list.version.clone())
            };
            let loader = if list.loader.is_empty() {
                None
            } else {
                Some(list.loader.clone())
            };
            let dir = if list.download_dir.is_empty() {
                None
            } else {
                Some(list.download_dir.clone())
            };
            (ver, loader, dir)
        } else {
            (None, None, None)
        }
    }

    pub fn get_effective_version(&self) -> String {
        self.get_current_list_settings()
            .0
            .unwrap_or_else(|| self.selected_version.clone())
    }

    pub fn get_effective_loader(&self) -> String {
        self.get_current_list_settings()
            .1
            .unwrap_or_else(|| self.selected_loader.clone())
    }

    pub fn get_effective_download_dir(&self) -> String {
        self.get_current_list_settings()
            .2
            .unwrap_or_else(|| self.download_dir.clone())
    }

    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::SearchResults(results) => {
                    self.search_window_results = results;
                }
                Event::ModDetails(mod_info) => {
                    let mod_id = mod_info.id.clone();
                    self.mods_being_loaded.remove(&mod_id);
                }
                Event::ModDetailsFailed { mod_id } => {
                    self.mods_being_loaded.remove(&mod_id);
                    self.mods_failed_loading.insert(mod_id);
                }
                Event::DownloadProgress { mod_id, progress } => {
                    if progress > 0.0 {
                        self.download_status
                            .insert(mod_id.clone(), DownloadStatus::Downloading);
                    }
                    self.download_progress.insert(mod_id, progress);
                }
                Event::DownloadComplete { mod_id, success } => {
                    self.download_status.insert(
                        mod_id,
                        if success {
                            DownloadStatus::Complete
                        } else {
                            DownloadStatus::Failed
                        },
                    );
                }
                Event::LegacyListProgress {
                    current,
                    total,
                    message,
                } => {
                    self.legacy_state = LegacyState::InProgress {
                        current,
                        total,
                        message,
                    };
                }
                Event::LegacyListComplete {
                    suggested_name,
                    successful,
                    failed,
                    warnings,
                    is_import: is_importable,
                } => {
                    let successful_ids = successful.iter().map(|m| m.id.clone()).collect();
                    self.pending_legacy_mods = Some(successful);
                    self.legacy_state = LegacyState::Complete {
                        suggested_name,
                        successful: successful_ids,
                        failed,
                        warnings,
                        is_import: is_importable,
                    };
                }
                Event::LegacyListFailed {
                    error,
                    is_import: is_importable,
                } => {
                    self.pending_legacy_mods = None;
                    self.legacy_state = LegacyState::Complete {
                        suggested_name: "".parse().unwrap(),
                        successful: Vec::new(),
                        failed: Vec::new(),
                        warnings: vec![error],
                        is_import: is_importable,
                    };
                }
            }
        }
    }

    pub fn get_current_list(&self) -> Option<&ModList> {
        self.current_list_id
            .as_ref()
            .and_then(|id| self.mod_lists.iter().find(|l| &l.id == id))
    }

    pub fn get_current_list_mut(&mut self) -> Option<&mut ModList> {
        let current_id = self.current_list_id.clone();
        current_id
            .as_ref()
            .and_then(|id| self.mod_lists.iter_mut().find(|l| &l.id == id))
    }

    pub fn get_filtered_mods(
        &self,
        query: &str,
        sort_mode: SortMode,
        order_mode: OrderMode,
        filter_mode: FilterMode,
    ) -> Vec<ModEntry> {
        let query = query.to_lowercase();
        let effective_version = self.get_effective_version();
        let effective_loader = self.get_effective_loader();

        if let Some(current_list) = self.get_current_list() {
            let mut mods: Vec<ModEntry> = current_list
                .mods
                .iter()
                .filter(|entry| {
                    let matches_search = entry.mod_name.to_lowercase().contains(&query)
                        || self
                            .get_mod_details(&entry.mod_id)
                            .map(|m| m.description.to_lowercase().contains(&query))
                            .unwrap_or(false);

                    let matches_filter = match filter_mode {
                        FilterMode::All => true,
                        FilterMode::CompatibleOnly => self
                            .check_mod_compatibility_with(
                                &entry.mod_id,
                                &effective_version,
                                &effective_loader,
                            )
                            .unwrap_or(false),
                        FilterMode::IncompatibleOnly => self
                            .check_mod_compatibility_with(
                                &entry.mod_id,
                                &effective_version,
                                &effective_loader,
                            )
                            .map(|c| !c)
                            .unwrap_or(false),
                    };

                    matches_search && matches_filter
                })
                .cloned()
                .collect();

            mods.sort_by(|a, b| match sort_mode {
                SortMode::Name => a.mod_name.cmp(&b.mod_name),
                SortMode::DateAdded => a.added_at.cmp(&b.added_at),
                SortMode::Compatibility => {
                    let comp_a = self
                        .check_mod_compatibility_with(
                            &a.mod_id,
                            &effective_version,
                            &effective_loader,
                        )
                        .unwrap_or(false);
                    let comp_b = self
                        .check_mod_compatibility_with(
                            &b.mod_id,
                            &effective_version,
                            &effective_loader,
                        )
                        .unwrap_or(false);
                    comp_b.cmp(&comp_a)
                }
            });

            if order_mode == OrderMode::Descending {
                mods.reverse();
            }

            mods
        } else {
            Vec::new()
        }
    }

    pub fn get_mod_details(&self, mod_id: &str) -> Option<Arc<ModInfo>> {
        self.mod_service.get_cached_mod_blocking(mod_id)
    }

    pub fn check_mod_compatibility(&self, mod_id: &str) -> Option<bool> {
        self.check_mod_compatibility_with(
            mod_id,
            &self.get_effective_version(),
            &self.get_effective_loader(),
        )
    }

    fn check_mod_compatibility_with(
        &self,
        mod_id: &str,
        version: &str,
        loader: &str,
    ) -> Option<bool> {
        self.get_mod_details(mod_id).map(|m| {
            m.supported_versions.contains(&version.to_string())
                && m.supported_loaders
                    .iter()
                    .any(|l| l.eq_ignore_ascii_case(loader))
        })
    }

    pub fn invalidate_and_reload(&mut self) {
        self.mod_service.clear_cache();
        self.mods_being_loaded.clear();
        self.mods_failed_loading.clear();

        let mod_ids: Vec<String> = if let Some(current_list) = self.get_current_list() {
            current_list.mods.iter().map(|e| e.mod_id.clone()).collect()
        } else {
            Vec::new()
        };

        for mod_id in mod_ids {
            self.load_mod_details_if_needed(&mod_id);
        }
    }

    pub fn load_mod_details_if_needed(&mut self, mod_id: &str) {
        if self.mods_being_loaded.contains(mod_id) {
            return;
        }
        if self.mods_failed_loading.contains(mod_id) {
            return;
        }

        if self.mod_service.contains_valid_blocking(mod_id) {
            return;
        }

        self.mods_being_loaded.insert(mod_id.to_string());
        let _ = self.cmd_tx.try_send(Command::FetchModDetails {
            mod_id: mod_id.to_string(),
            version: self.get_effective_version(),
            loader: self.get_effective_loader(),
        });
    }

    pub fn start_download(&mut self, mod_id: &str) {
        self.download_status
            .insert(mod_id.to_string(), DownloadStatus::Queued);
        self.download_progress.insert(mod_id.to_string(), 0.0);

        if let Some(mod_info) = self.get_mod_details(mod_id) {
            let _ = self.cmd_tx.try_send(Command::DownloadMod {
                mod_info,
                download_dir: self.get_effective_download_dir(),
            });
        }
    }

    pub fn perform_search(&mut self, query: &str) {
        if !query.is_empty() {
            let _ = self.cmd_tx.try_send(Command::SearchMods {
                query: query.to_string(),
                version: if self.search_filter_exact {
                    self.get_effective_version()
                } else {
                    String::new()
                },
                loader: if self.search_filter_exact {
                    self.get_effective_loader()
                } else {
                    String::new()
                },
            });
        }
    }

    pub fn add_mod_to_current_list(&mut self, mod_info: Arc<ModInfo>) {
        if let Some(current_list) = self.get_current_list_mut() {
            if !current_list.mods.iter().any(|e| e.mod_id == mod_info.id) {
                current_list.mods.push(ModEntry {
                    mod_id: mod_info.id.clone(),
                    mod_name: mod_info.name.clone(),
                    added_at: Utc::now(),
                    archived: false,
                });
                self.download_status
                    .insert(mod_info.id.clone(), DownloadStatus::Idle);
            }
        }
    }

    pub fn delete_mod(&mut self, mod_id: &str) {
        if let Some(current_list) = self.get_current_list_mut() {
            current_list.mods.retain(|e| e.mod_id != mod_id);
        }

        self.mods_being_loaded.remove(mod_id);
        self.mods_failed_loading.remove(mod_id);
        self.download_progress.remove(mod_id);
        self.download_status.remove(mod_id);

        if let Some(current_list) = self.get_current_list() {
            let list = current_list.clone();
            let cm = self.config_manager.clone();
            self.runtime_handle.spawn(async move {
                let _ = cm.save_list(&list).await;
            });
        }
    }

    pub fn toggle_archive_mod(&mut self, mod_id: &str) {
        if let Some(list) = self.get_current_list_mut() {
            if let Some(entry) = list.mods.iter_mut().find(|e| e.mod_id == mod_id) {
                entry.archived = !entry.archived;
                let list = list.clone();
                let cm = self.config_manager.clone();
                self.runtime_handle.spawn(async move {
                    let _ = cm.save_list(&list).await;
                });
            }
        }
    }

    pub fn update_list_settings(&mut self, mut version: String, mut loader: String, dir: String) {
        if version == "default" {
            version = String::new();
        }
        if loader == "default" {
            loader = String::new();
        }

        if let Some(list) = self.get_current_list_mut() {
            list.version = version;
            list.loader = loader;
            list.download_dir = dir;

            let list_clone = list.clone();
            let cm = self.config_manager.clone();

            self.runtime_handle.spawn(async move {
                let _ = cm.save_list(&list_clone).await;
            });
        }

        if self.current_list_id.is_some() {
            self.invalidate_and_reload();
        }
    }

    pub fn delete_current_list(&mut self) {
        if let Some(list_id) = self.current_list_id.clone() {
            self.mod_lists.retain(|l| l.id != list_id);
            self.current_list_id = None;
            let cm = self.config_manager.clone();
            self.runtime_handle.spawn(async move {
                let _ = cm.delete_list(&list_id).await;
            });
        }
    }

    pub fn save_list(&self, list: &ModList) {
        let cm = self.config_manager.clone();
        let list = list.clone();
        self.runtime_handle.spawn(async move {
            let _ = cm.save_list(&list).await;
        });
    }

    pub fn update_download_dir(&mut self, new_dir: String) {
        self.download_dir = new_dir.clone();
        let cm = self.config_manager.clone();
        self.runtime_handle.spawn(async move {
            if let Ok(mut config) = cm.load_config().await {
                config.download_dir = new_dir;
                let _ = cm.save_config(&config).await;
            }
        });
    }

    pub fn export_current_list(&mut self, path: std::path::PathBuf) {
        let export_info = self.get_current_list().map(|list| {
            (
                list.mods
                    .iter()
                    .map(|m| m.mod_id.clone())
                    .collect::<Vec<String>>(),
                list.clone(),
            )
        });

        let (mod_ids, current_list_obj) = match export_info {
            Some(data) => data,
            None => return,
        };

        match path.extension().and_then(|s| s.to_str()) {
            Some("mods") => {
                self.legacy_state = LegacyState::InProgress {
                    current: 0,
                    total: mod_ids.len(),
                    message: "Initializing export...".into(),
                };

                let _ = self.cmd_tx.try_send(Command::LegacyListExport {
                    path,
                    mod_ids,
                    version: self.get_effective_version(),
                    loader: self.get_effective_loader(),
                });
            }
            _ => {
                let runtime = self.runtime_handle.clone();
                runtime.spawn(async move {
                    let toml_string = toml::to_string_pretty(&current_list_obj).unwrap_or_default();
                    let _ = tokio::fs::write(path, toml_string).await;
                });
            }
        }
    }

    pub fn start_legacy_import(&mut self, path: std::path::PathBuf) {
        self.legacy_state = LegacyState::InProgress {
            current: 0,
            total: 0,
            message: "Preparing import...".into(),
        };
        let _ = self.cmd_tx.try_send(Command::LegacyListImport {
            path,
            version: self.get_effective_version(),
            loader: self.get_effective_loader(),
        });
    }

    pub fn finalize_import(&mut self, list: ModList) {
        let mut list_to_save = list;
        if list_to_save.version.is_empty() {
            list_to_save.version = self.selected_version.clone();
        }
        if list_to_save.loader.is_empty() {
            list_to_save.loader = self.selected_loader.clone();
        }
        if list_to_save.download_dir.is_empty() {
            list_to_save.download_dir = self.download_dir.clone();
        }

        let cm = self.config_manager.clone();
        let list_for_spawn = list_to_save.clone();
        self.runtime_handle.spawn(async move {
            let _ = cm.save_list(&list_for_spawn).await;
        });

        self.current_list_id = Some(list_to_save.id.clone());
        self.mod_lists.push(list_to_save);
    }

    pub fn persist_config_on_exit(&self) {
        let cm = self.config_manager.clone();
        let config = AppConfig {
            selected_version: self.selected_version.clone(),
            selected_loader: self.selected_loader.clone(),
            current_list_id: self.current_list_id.clone(),
            download_dir: self.download_dir.clone(),
        };

        if let Some(current_list) = self.get_current_list() {
            let list = current_list.clone();
            self.runtime_handle.block_on(async {
                let _ = cm.save_config(&config).await;
                let _ = cm.save_list(&list).await;
            });
        } else {
            self.runtime_handle.block_on(async {
                let _ = cm.save_config(&config).await;
            });
        }
    }

    pub fn create_new_list(&mut self) {
        let new_list = ModList {
            id: format!("list_{}", Utc::now().timestamp()),
            name: "New List".to_string(),
            created_at: Utc::now(),
            mods: Vec::new(),
            version: self.selected_version.clone(),
            loader: self.selected_loader.clone(),
            download_dir: self.download_dir.clone(),
        };

        self.save_list(&new_list);
        self.current_list_id = Some(new_list.id.clone());
        self.mod_lists.push(new_list);
    }
}
