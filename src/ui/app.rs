use crate::adapters::ModrinthProvider;
use crate::domain::*;
use crate::domain::{AppConfig, ModList, ModService};
use crate::infra::LegacyListService;
use crate::infra::*;
use crate::ui::app::ListAction::Import;
use chrono::Utc;
use eframe::egui;
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

#[derive(PartialEq)]
pub enum ListAction {
    Import,
    Duplicate,
}

#[derive(PartialEq)]
enum LegacyState {
    Idle,
    InProgress {
        current: usize,
        total: usize,
        message: String,
    },
    Complete {
        successful: Vec<String>,
        failed: Vec<String>,
        warnings: Vec<String>,
        is_import: bool,
    },
}

pub struct App {
    mod_service: Arc<ModService>,
    config_manager: Arc<ConfigManager>,
    minecraft_versions: Vec<MinecraftVersion>,
    mod_loaders: Vec<ModLoader>,
    selected_version: String,
    selected_loader: String,
    previous_version: String,
    previous_loader: String,
    mod_lists: Vec<ModList>,
    current_list_id: Option<String>,
    filtered_mods: Vec<ModEntry>,
    list_search_query: String,
    search_query: String,
    selected_mod: Option<usize>,
    download_progress: HashMap<String, f32>,
    download_status: HashMap<String, DownloadStatus>,
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
    search_window_open: bool,
    search_window_query: String,
    search_window_results: Vec<Arc<ModInfo>>,
    rename_list_input: String,
    show_rename_input: bool,
    settings_window_open: bool,
    mods_being_loaded: HashSet<String>,
    mods_failed_loading: HashSet<String>,
    download_dir: String,
    _runtime: tokio::runtime::Runtime,
    runtime_handle: tokio::runtime::Handle,
    import_window_open: bool,
    pending_import_list: Option<ModList>,
    import_name_input: String,
    active_action: ListAction,
    legacy_state: LegacyState,
    pending_legacy_mods: Option<Vec<Arc<ModInfo>>>,
    legacy_service: Arc<LegacyListService>,
    // Launcher fields
    java_installations: Vec<JavaInstallation>,
    selected_java_index: Option<usize>,
    minecraft_installation: Option<MinecraftInstallation>,
    launcher_username: String,
    launcher_min_memory: u32,
    launcher_max_memory: u32,
    launch_status: Option<String>,
    active_tab: usize,
    selected_mc_version: String,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let runtime_handle = runtime.handle().clone();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(100);
        let (event_tx, event_rx) = mpsc::channel::<Event>(100);

        let provider: Arc<dyn ModProvider> = Arc::new(ModrinthProvider::new());
        let provider_clone = provider.clone();

        let connection_limiter = Arc::new(ConnectionLimiter::new(5));
        let limiter_clone = connection_limiter.clone();

        let mod_pool = Arc::new(Mutex::new(ModInfoPool::new(500, 1)));
        let pool_clone_for_spawn = mod_pool.clone();

        let mod_service = Arc::new(ModService::new(
            provider_clone.clone(),
            connection_limiter,
            pool_clone_for_spawn.clone(),
        ));
        let mod_service_for_spawn = mod_service.clone();

        let legacy_service = Arc::new(LegacyListService::new(mod_service.clone()));
        let legacy_service_for_spawn = legacy_service.clone();

        runtime_handle.spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let provider = provider_clone.clone();
                let event_tx = event_tx.clone();
                let limiter = limiter_clone.clone();
                let legacy_service = legacy_service_for_spawn.clone();
                let mod_service = mod_service_for_spawn.clone();

                match cmd {
                    Command::SearchMods {
                        query,
                        version,
                        loader,
                    } => {
                        tokio::spawn(async move {
                            let _permit = limiter.acquire(1).await;

                            if let Ok(results) =
                                provider.search_mods(&query, &version, &loader).await
                            {
                                let cached_results =
                                    mod_service.cache_search_results(results).await;

                                let _ = event_tx.send(Event::SearchResults(cached_results)).await;
                            }
                        });
                    }
                    Command::FetchModDetails {
                        mod_id,
                        version,
                        loader,
                    } => {
                        tokio::spawn(async move {
                            match mod_service.get_mod_by_id(&mod_id, &version, &loader).await {
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
                            let _permit = limiter.acquire(3).await;

                            let mod_id = mod_info.id.clone();
                            let filename = format!("{}.jar", mod_info.name.replace(" ", "_"));
                            let destination = std::path::Path::new(&download_dir).join(&filename);

                            let tx_progress = event_tx.clone();
                            let mod_id_clone = mod_id.clone();

                            let result = provider
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
                            legacy_service
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
                            legacy_service
                                .export_legacy_list(path, mod_ids, version, loader, event_tx)
                                .await;
                        });
                    }
                }
            }
        });

        let config_manager =
            Arc::new(ConfigManager::new().expect("Failed to create config manager"));

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
            let prov = provider.clone();

            runtime_handle.block_on(async {
                let _ = cm.ensure_dirs().await;

                let config = if cm.config_exists() {
                    match cm.load_config().await {
                        Ok(cfg) => cfg,
                        Err(_) => cm.create_default_config().await.unwrap(),
                    }
                } else {
                    cm.create_default_config().await.unwrap()
                };

                let lists = match cm.load_all_lists().await {
                    Ok(lists) => lists,
                    Err(_) => Vec::new(),
                };

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
            filtered_mods: Vec::new(),
            list_search_query: String::new(),
            search_query: String::new(),
            selected_mod: None,
            download_progress: HashMap::new(),
            download_status: HashMap::new(),
            cmd_tx,
            event_rx,
            search_window_open: false,
            search_window_query: String::new(),
            search_window_results: Vec::new(),
            rename_list_input: String::new(),
            show_rename_input: false,
            settings_window_open: false,
            mods_being_loaded: HashSet::new(),
            mods_failed_loading: HashSet::new(),
            download_dir,
            _runtime: runtime,
            runtime_handle,
            import_window_open: false,
            pending_import_list: None,
            import_name_input: String::new(),
            active_action: Import,
            legacy_state: LegacyState::Idle,
            pending_legacy_mods: None,
            legacy_service,
            // Initialize launcher fields
            java_installations: JavaDetector::detect_java_installations(),
            selected_java_index: None,
            minecraft_installation: {
                let mc = MinecraftDetector::detect_minecraft();
                mc
            },
            launcher_username: whoami::username(),
            launcher_min_memory: 1024,
            launcher_max_memory: 4096,
            launch_status: None,
            active_tab: 0,
            selected_mc_version: MinecraftDetector::detect_minecraft()
                .and_then(|mc| mc.available_versions.first().cloned())
                .unwrap_or_else(|| "1.20.1".to_string()),
        }
    }

    fn invalidate_and_reload(&mut self) {
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

    fn get_current_list(&self) -> Option<&ModList> {
        self.current_list_id
            .as_ref()
            .and_then(|id| self.mod_lists.iter().find(|l| &l.id == id))
    }

    fn get_current_list_mut(&mut self) -> Option<&mut ModList> {
        let current_id = self.current_list_id.clone();
        current_id
            .as_ref()
            .and_then(|id| self.mod_lists.iter_mut().find(|l| &l.id == id))
    }

    fn filter_mods(&mut self) {
        let query = self.search_query.to_lowercase();
        if let Some(current_list) = self.get_current_list() {
            self.filtered_mods = current_list
                .mods
                .iter()
                .filter(|entry| {
                    entry.mod_name.to_lowercase().contains(&query)
                        || self
                            .get_mod_details(&entry.mod_id)
                            .map(|m| m.description.to_lowercase().contains(&query))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
        } else {
            self.filtered_mods.clear();
        }
    }

    fn get_mod_details(&self, mod_id: &str) -> Option<Arc<ModInfo>> {
        self.mod_service.get_cached_mod_blocking(mod_id)
    }

    fn is_mod_loading(&self, mod_id: &str) -> bool {
        self.mods_being_loaded.contains(mod_id)
    }

    fn load_mod_details_if_needed(&mut self, mod_id: &str) {
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
            version: self.selected_version.clone(),
            loader: self.selected_loader.clone(),
        });
    }

    fn start_download(&mut self, mod_id: &str) {
        self.download_status
            .insert(mod_id.to_string(), DownloadStatus::Queued);
        self.download_progress.insert(mod_id.to_string(), 0.0);

        if let Some(mod_info) = self.get_mod_details(mod_id) {
            let _ = self.cmd_tx.try_send(Command::DownloadMod {
                mod_info,
                download_dir: self.download_dir.clone(),
            });
        }
    }

    fn handle_events(&mut self) {
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
                    successful,
                    failed,
                    warnings,
                    is_import: is_importable,
                } => {
                    let successful_ids = successful.iter().map(|m| m.id.clone()).collect();
                    self.pending_legacy_mods = Some(successful);
                    self.legacy_state = LegacyState::Complete {
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
                        successful: Vec::new(),
                        failed: Vec::new(),
                        warnings: vec![error],
                        is_import: is_importable,
                    };
                }
            }
        }
    }

    fn check_mod_compatibility(&self, mod_id: &str) -> Option<bool> {
        self.get_mod_details(mod_id).map(|m| {
            m.supported_versions.contains(&self.selected_version)
                && m.supported_loaders.contains(&self.selected_loader)
        })
    }

    fn perform_search(&mut self) {
        if !self.search_window_query.is_empty() {
            let _ = self.cmd_tx.try_send(Command::SearchMods {
                query: self.search_window_query.clone(),
                version: self.selected_version.clone(),
                loader: self.selected_loader.clone(),
            });
        }
    }

    fn delete_mod(&mut self, mod_id: &str) {
        if let Some(current_list) = self.get_current_list_mut() {
            current_list.mods.retain(|e| e.mod_id != mod_id);
        }

        self.filtered_mods.retain(|e| e.mod_id != mod_id);
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

    fn add_mod_to_current_list(&mut self, mod_info: Arc<ModInfo>) {
        if let Some(current_list) = self.get_current_list_mut() {
            if !current_list.mods.iter().any(|e| e.mod_id == mod_info.id) {
                current_list.mods.push(ModEntry {
                    mod_id: mod_info.id.clone(),
                    mod_name: mod_info.name.clone(),
                    added_at: Utc::now(),
                });
                self.download_status
                    .insert(mod_info.id.clone(), DownloadStatus::Idle);
            }
        }
        self.search_window_open = false;
        self.search_window_query.clear();
        self.search_window_results.clear();
    }

    fn delete_current_list(&mut self) {
        if let Some(list_id) = self.current_list_id.clone() {
            self.mod_lists.retain(|l| l.id != list_id);
            self.current_list_id = None;
            self.selected_mod = None;
            let cm = self.config_manager.clone();
            self.runtime_handle.spawn(async move {
                let _ = cm.delete_list(&list_id).await;
            });
        }
    }

    fn export_current_list(&mut self) {
        let export_info = self.get_current_list().map(|list| {
            (
                list.name.clone(),
                list.mods
                    .iter()
                    .map(|m| m.mod_id.clone())
                    .collect::<Vec<String>>(),
                list.clone(),
            )
        });

        let (list_name, mod_ids, current_list_obj) = match export_info {
            Some(data) => data,
            None => return,
        };

        if let Some(save_path) = FileDialog::new()
            .add_filter("TOML Config", &["toml"])
            .add_filter("Legacy Mod List", &["mods"])
            .set_title("Export Mod List")
            .set_file_name(&format!("{}.toml", list_name))
            .save_file()
        {
            match save_path.extension().and_then(|s| s.to_str()) {
                Some("mods") => {
                    self.legacy_state = LegacyState::InProgress {
                        current: 0,
                        total: mod_ids.len(),
                        message: "Initializing export...".into(),
                    };

                    let _ = self.cmd_tx.try_send(Command::LegacyListExport {
                        path: save_path,
                        mod_ids,
                        version: self.selected_version.clone(),
                        loader: self.selected_loader.clone(),
                    });
                }
                _ => {
                    let runtime = self.runtime_handle.clone();
                    runtime.spawn(async move {
                        let toml_string =
                            toml::to_string_pretty(&current_list_obj).unwrap_or_default();
                        let _ = tokio::fs::write(save_path, toml_string).await;
                    });
                }
            }
        }
    }

    fn finalize_import(&mut self) {
        if let Some(mut list) = self.pending_import_list.take() {
            list.id = format!("list_{}", chrono::Utc::now().timestamp_millis());
            list.name = self.import_name_input.trim().to_string();

            if list.name.is_empty() {
                list.name = "Unnamed List".to_string();
            }

            let cm = self.config_manager.clone();
            let list_to_save = list.clone();
            self.runtime_handle.spawn(async move {
                let _ = cm.save_list(&list_to_save).await;
            });

            self.current_list_id = Some(list.id.clone());
            self.mod_lists.push(list);

            self.import_window_open = false;
            self.import_name_input.clear();
        }
    }

    fn render_launcher_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Minecraft Launcher");
        ui.add_space(10.0);

        // Java installation section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Java Installation").strong());
            ui.add_space(5.0);

            if self.java_installations.is_empty() {
                ui.colored_label(egui::Color32::RED, "⚠ No Java installations found!");
                ui.label("Please install Java 17+ to launch Minecraft.");
            } else {
                egui::ComboBox::from_label("Select Java")
                    .selected_text(
                        self.selected_java_index
                            .and_then(|idx| self.java_installations.get(idx))
                            .map(|j| format!("{} ({})", j.version, j.path.display()))
                            .unwrap_or_else(|| "Select Java...".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for (idx, java) in self.java_installations.iter().enumerate() {
                            let label = format!("{} - {}", java.version, java.path.display());
                            ui.selectable_value(&mut self.selected_java_index, Some(idx), label);
                        }
                    });

                if let Some(idx) = self.selected_java_index {
                    if let Some(java) = self.java_installations.get(idx) {
                        if java.is_valid {
                            ui.colored_label(egui::Color32::GREEN, "✓ Valid Java installation");
                        } else {
                            ui.colored_label(egui::Color32::YELLOW, "⚠ Java validation failed");
                        }
                    }
                }
            }
        });

        ui.add_space(10.0);

        // Minecraft installation section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Minecraft Installation").strong());
            ui.add_space(5.0);

            if let Some(ref mc) = self.minecraft_installation {
                ui.colored_label(egui::Color32::GREEN, "✓ Minecraft found");
                ui.label(format!("Location: {}", mc.root_dir.display()));
                ui.label(format!("Installed versions: {}", mc.available_versions.len()));

                ui.add_space(5.0);

                if !mc.available_versions.is_empty() {
                    egui::ComboBox::from_label("Select Minecraft Version")
                        .selected_text(&self.selected_mc_version)
                        .show_ui(ui, |ui| {
                            for version in &mc.available_versions {
                                ui.selectable_value(&mut self.selected_mc_version, version.clone(), version);
                            }
                        });
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "⚠ No Minecraft versions installed");
                }
            } else {
                ui.colored_label(egui::Color32::RED, "⚠ Minecraft not found!");
                ui.label("Please install Minecraft to use the launcher.");
            }
        });

        ui.add_space(10.0);

        // Launch settings
        ui.group(|ui| {
            ui.label(egui::RichText::new("Launch Settings").strong());
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.text_edit_singleline(&mut self.launcher_username);
            });

            ui.add_space(5.0);

            ui.label(format!("Minimum Memory: {} MB", self.launcher_min_memory));
            ui.add(egui::Slider::new(&mut self.launcher_min_memory, 512..=8192).suffix(" MB"));

            ui.add_space(5.0);

            ui.label(format!("Maximum Memory: {} MB", self.launcher_max_memory));
            ui.add(egui::Slider::new(&mut self.launcher_max_memory, 1024..=16384).suffix(" MB"));

            if self.launcher_min_memory > self.launcher_max_memory {
                ui.colored_label(egui::Color32::RED, "⚠ Min memory cannot be greater than max memory!");
            }
        });

        ui.add_space(10.0);

        // Mod list selection
        ui.group(|ui| {
            ui.label(egui::RichText::new("Mod List").strong());
            ui.add_space(5.0);

            if let Some(list_id) = &self.current_list_id {
                if let Some(list) = self.mod_lists.iter().find(|l| &l.id == list_id) {
                    ui.colored_label(egui::Color32::GREEN, format!("✓ Using list: {}", list.name));
                    ui.label(format!("Mods: {}", list.mods.len()));
                } else {
                    ui.label("No list selected");
                }
            } else {
                ui.label("No list selected - will launch vanilla Minecraft");
            }
        });

        ui.add_space(15.0);

        // Launch button
        let has_valid_mc_version = self.minecraft_installation
            .as_ref()
            .map(|mc| mc.available_versions.contains(&self.selected_mc_version))
            .unwrap_or(false);

        let can_launch = !self.java_installations.is_empty()
            && self.selected_java_index.is_some()
            && self.minecraft_installation.is_some()
            && has_valid_mc_version
            && !self.launcher_username.is_empty()
            && self.launcher_min_memory <= self.launcher_max_memory;

        ui.horizontal(|ui| {
            let launch_button = egui::Button::new(egui::RichText::new("🚀 Launch Minecraft").size(18.0));

            if ui.add_sized([200.0, 40.0], launch_button)
                .on_disabled_hover_text("Configure Java and Minecraft to launch")
                .on_hover_text("Launch Minecraft with selected settings")
                .clicked() && can_launch
            {
                self.launch_minecraft();
            }
        });

        // Status message
        if let Some(ref status) = self.launch_status {
            ui.add_space(10.0);
            ui.separator();
            ui.label(status);
        }
    }

    fn launch_minecraft(&mut self) {
        self.launch_status = Some("Preparing to launch...".to_string());

        let java_idx = match self.selected_java_index {
            Some(idx) => idx,
            None => {
                self.launch_status = Some("❌ No Java selected".to_string());
                return;
            }
        };

        let java = match self.java_installations.get(java_idx) {
            Some(j) => j,
            None => {
                self.launch_status = Some("❌ Invalid Java selection".to_string());
                return;
            }
        };

        let mc_install = match &self.minecraft_installation {
            Some(mc) => mc,
            None => {
                self.launch_status = Some("❌ Minecraft not found".to_string());
                return;
            }
        };

        // Copy mods if a list is selected
        if let Some(list_id) = &self.current_list_id {
            if let Some(list) = self.mod_lists.iter().find(|l| &l.id == list_id) {
                let mod_names: Vec<String> = list.mods.iter().map(|m| m.mod_name.clone()).collect();
                let source_dir = std::path::PathBuf::from(&self.download_dir);
                let mods_dir = mc_install.mods_dir.clone();

                let runtime = self.runtime_handle.clone();
                let status_msg = format!("Copying {} mods...", mod_names.len());
                self.launch_status = Some(status_msg);

                runtime.spawn(async move {
                    let _ = ModCopier::copy_mods_to_minecraft(&source_dir, &mods_dir, &mod_names).await;
                });
            }
        }

        // Build launch config
        let config = LaunchConfig {
            profile: LaunchProfile {
                minecraft_version: self.selected_mc_version.clone(),
                mod_loader: self.selected_loader.clone(),
                mod_loader_version: None,
                java_path: java.path.clone(),
                game_directory: mc_install.root_dir.clone(),
                mod_list_id: self.current_list_id.clone(),
            },
            username: self.launcher_username.clone(),
            max_memory_mb: self.launcher_max_memory,
            min_memory_mb: self.launcher_min_memory,
        };

        // Launch Minecraft
        match LauncherService::launch_minecraft(&config) {
            Ok(LaunchResult::Success { pid }) => {
                self.launch_status = Some(format!("✅ Minecraft launched successfully! (PID: {})", pid));
            }
            Ok(LaunchResult::Failed { error }) => {
                self.launch_status = Some(format!("❌ Launch failed: {}", error));
            }
            Err(e) => {
                self.launch_status = Some(format!("❌ Error: {}", e));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.settings_window_open = false;
            self.search_window_open = false;
            self.import_window_open = false;
            self.pending_import_list = None;
        }

        if self.search_window_open && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.perform_search();
        }

        if self.import_window_open && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.finalize_import();
        }

        if self.settings_window_open {
            let overlay = egui::Area::new(egui::Id::new("settings_overlay"))
                .order(egui::Order::Background)
                .fixed_pos(egui::pos2(0.0, 0.0));

            overlay.show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

                if ui
                    .interact(
                        screen_rect,
                        egui::Id::new("settings_overlay_click"),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    self.settings_window_open = false;
                }
            });

            let mut open = self.settings_window_open;
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Download Directory:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.download_dir);
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = FileDialog::new()
                                .set_title("Select Download Directory")
                                .pick_folder()
                            {
                                self.download_dir = path.to_string_lossy().to_string();
                                let cm = self.config_manager.clone();
                                let dir = self.download_dir.clone();
                                self.runtime_handle.spawn(async move {
                                    if let Ok(mut config) = cm.load_config().await {
                                        config.download_dir = dir;
                                        let _ = cm.save_config(&config).await;
                                    }
                                });
                            }
                        }
                    });
                });
            self.settings_window_open = open;
        }

        if self.import_window_open && self.pending_import_list.is_some() {
            let overlay_id = egui::Id::new("import_overlay");
            let overlay = egui::Area::new(overlay_id)
                .order(egui::Order::Background)
                .fixed_pos(egui::pos2(0.0, 0.0));

            overlay.show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

                if ui
                    .interact(screen_rect, overlay_id.with("click"), egui::Sense::click())
                    .clicked()
                {
                    self.import_window_open = false;
                    self.pending_import_list = None;
                }
            });

            let mod_count = self
                .pending_import_list
                .as_ref()
                .map(|l| l.mods.len())
                .unwrap_or(0);

            let title = match self.active_action {
                ListAction::Import => "Import Mod List",
                ListAction::Duplicate => "Duplicate Mod List",
            };

            let mut should_finalize = false;
            let mut should_close = false;
            let mut open = self.import_window_open;

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("List Name:");
                    ui.text_edit_singleline(&mut self.import_name_input);

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(format!("Contains {} mods", mod_count)).weak());

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Confirm").clicked() {
                            should_finalize = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });

            if should_close || !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.import_window_open = false;
                self.pending_import_list = None;
            } else if should_finalize
                || (ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.show_rename_input)
            {
                self.finalize_import();
            }
        }

        if self.legacy_state != LegacyState::Idle {
            let mut is_open = true;
            let overlay = egui::Area::new(egui::Id::new("legacy_overlay"))
                .order(egui::Order::Background)
                .fixed_pos(egui::pos2(0.0, 0.0));

            overlay.show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

                if ui
                    .interact(
                        screen_rect,
                        egui::Id::new("legacy_overlay_click"),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    if matches!(self.legacy_state, LegacyState::Complete { .. }) {
                        is_open = false;
                    }
                }
            });

            let mut should_import = false;

            let window_title = match &self.legacy_state {
                LegacyState::InProgress { .. } => "Processing...",
                _ => "Operation Complete",
            };

            egui::Window::new(window_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut is_open)
                .show(ctx, |ui| {
                    ui.set_min_width(300.0);

                    match &self.legacy_state {
                        LegacyState::InProgress {
                            current,
                            total,
                            message,
                        } => {
                            let progress = if *total > 0 {
                                *current as f32 / *total as f32
                            } else {
                                0.0
                            };
                            let current_val = *current;
                            let total_val = *total;
                            let msg = message.clone();

                            ui.vertical_centered(|ui| {
                                ui.add_space(10.0);
                                ui.add(egui::Spinner::new().size(32.0));
                                ui.add_space(10.0);
                                ui.label(msg);
                                ui.add(
                                    egui::ProgressBar::new(progress)
                                        .text(format!("{}/{}", current_val, total_val)),
                                );
                                ui.add_space(10.0);
                            });
                        }
                        LegacyState::Complete {
                            successful,
                            failed,
                            warnings,
                            is_import: is_import,
                        } => {
                            let success_count = successful.len();
                            let fail_count = failed.len();
                            let warn_count = warnings.len();
                            let is_importable = self.pending_legacy_mods.is_some();

                            ui.vertical(|ui| {
                                ui.heading(if *is_import {
                                    "Import Results"
                                } else {
                                    "Export Results"
                                });
                                ui.label(format!("✅ Success: {}", success_count));

                                if fail_count > 0 {
                                    ui.colored_label(
                                        egui::Color32::LIGHT_RED,
                                        format!("❌ Failed: {}", fail_count),
                                    );
                                }
                                if warn_count > 0 {
                                    ui.colored_label(
                                        egui::Color32::GOLD,
                                        format!("⚠️ Warnings: {}", warn_count),
                                    );
                                }

                                if *is_import && is_importable && success_count > 0 {
                                    ui.add_space(15.0);
                                    ui.separator();
                                    ui.add_space(10.0);
                                    ui.horizontal(|ui| {
                                        if ui.button("📥 Import into List").clicked() {
                                            should_import = true;
                                        }
                                    });
                                }
                            });
                        }
                        _ => {}
                    }
                });

            if should_import {
                if let Some(mods) = self.pending_legacy_mods.take() {
                    let entries = mods
                        .into_iter()
                        .map(|m| ModEntry {
                            mod_id: m.id.clone(),
                            mod_name: m.name.clone(),
                            added_at: Utc::now(),
                        })
                        .collect();

                    self.pending_import_list = Some(ModList {
                        id: format!("list_{}", Utc::now().timestamp()),
                        name: "Imported List".to_string(),
                        created_at: Utc::now(),
                        mods: entries,
                    });
                    self.import_name_input = "Imported List".to_string();
                    self.active_action = Import;
                    self.import_window_open = true;
                }
                self.legacy_state = LegacyState::Idle;
            } else if !is_open {
                self.legacy_state = LegacyState::Idle;
                self.pending_legacy_mods = None;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Minecraft Mod Downloader");
                ui.separator();

                ui.label("Version:");
                let version_changed = egui::ComboBox::from_id_source("version_selector")
                    .selected_text(&self.selected_version)
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for version in &self.minecraft_versions {
                            if ui
                                .selectable_value(
                                    &mut self.selected_version,
                                    version.id.clone(),
                                    &version.name,
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);

                ui.label("Loader:");
                let loader_changed = egui::ComboBox::from_id_source("loader_selector")
                    .selected_text(&self.selected_loader)
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for loader in &self.mod_loaders {
                            if ui
                                .selectable_value(
                                    &mut self.selected_loader,
                                    loader.id.clone(),
                                    &loader.name,
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);

                if version_changed || loader_changed {
                    self.invalidate_and_reload();
                    self.previous_version = self.selected_version.clone();
                    self.previous_loader = self.selected_loader.clone();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        self.settings_window_open = true;
                    }
                });
            });
        });

        egui::SidePanel::left("sidebar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.list_search_query)
                    .hint_text("🔍 Search mod lists...")
                    .desired_width(ui.available_width()),
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let button_width = ui.available_width() - 35.0;
                if ui
                    .add_sized([button_width, 25.0], egui::Button::new("➕ New List"))
                    .clicked()
                {
                    let new_list = ModList {
                        id: format!("list_{}", chrono::Utc::now().timestamp()),
                        name: "New List".to_string(),
                        created_at: Utc::now(),
                        mods: Vec::new(),
                    };

                    let cm = self.config_manager.clone();
                    let list_to_save = new_list.clone();
                    self.runtime_handle.spawn(async move {
                        let _ = cm.save_list(&list_to_save).await;
                    });

                    self.current_list_id = Some(new_list.id.clone());
                    self.mod_lists.push(new_list);
                }

                if ui
                    .add_sized([25.0, 25.0], egui::Button::new("📥"))
                    .on_hover_text("Import")
                    .clicked()
                {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Mod List Files", &["toml", "mods"])
                        .pick_file()
                    {
                        match path.extension().and_then(|s| s.to_str()) {
                            Some("toml") => {
                                if let Ok(content) = std::fs::read_to_string(path) {
                                    if let Ok(list) = toml::from_str::<ModList>(&content) {
                                        self.import_name_input =
                                            format!("{} (Imported)", list.name);
                                        self.pending_import_list = Some(list);
                                        self.active_action = ListAction::Import;
                                        self.import_window_open = true;
                                    }
                                }
                            }
                            Some("mods") => {
                                self.legacy_state = LegacyState::InProgress {
                                    current: 0,
                                    total: 0,
                                    message: "Preparing export...".into(),
                                };
                                let _ = self.cmd_tx.try_send(Command::LegacyListImport {
                                    path,
                                    version: self.selected_version.clone(),
                                    loader: self.selected_loader.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            });

            ui.add_space(4.0);
            ui.separator();

            let filtered_lists: Vec<_> = self
                .mod_lists
                .iter()
                .filter(|list| {
                    self.list_search_query.is_empty()
                        || list
                            .name
                            .to_lowercase()
                            .contains(&self.list_search_query.to_lowercase())
                })
                .collect();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for list in filtered_lists {
                    let selected = self.current_list_id.as_ref() == Some(&list.id);
                    if ui
                        .selectable_label(selected, format!("{} ({})", list.name, list.mods.len()))
                        .clicked()
                    {
                        if selected {
                            self.current_list_id = None;
                        } else {
                            self.current_list_id = Some(list.id.clone());
                        }
                        self.selected_mod = None;
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab selection
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, 0, "📦 Mod Lists");
                ui.selectable_value(&mut self.active_tab, 1, "🚀 Launcher");
            });
            ui.separator();
            ui.add_space(5.0);

            // Render appropriate tab
            if self.active_tab == 1 {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_launcher_tab(ui);
                });
                return;
            }

            // Original mod list UI (tab 0)
            if self.current_list_id.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("No list selected");
                    ui.label("Select a list from the sidebar or create a new one");
                });
                return;
            }

            let can_interact = self.current_list_id.is_some();

            ui.horizontal(|ui| {
                if self.show_rename_input {
                    ui.text_edit_singleline(&mut self.rename_list_input);
                    if ui.button("✔").clicked() {
                        let new_name = self.rename_list_input.clone();
                        if let Some(list) = self.get_current_list_mut() {
                            list.name = new_name;
                            let list_clone = list.clone();
                            let cm = self.config_manager.clone();
                            self.runtime_handle.spawn(async move {
                                let _ = cm.save_list(&list_clone).await;
                            });
                        }
                        self.show_rename_input = false;
                    }
                    if ui.button("❌").clicked() {
                        self.show_rename_input = false;
                    }
                } else {
                    if ui
                        .add_enabled(can_interact, egui::Button::new("✏ Rename"))
                        .clicked()
                    {
                        self.show_rename_input = true;
                        if let Some(list) = self.get_current_list() {
                            self.rename_list_input = list.name.clone();
                        }
                    }

                    if ui
                        .add_enabled(can_interact, egui::Button::new("👥 Duplicate"))
                        .clicked()
                    {
                        let list_to_dup = self.get_current_list().cloned();

                        if let Some(list) = list_to_dup {
                            self.import_name_input = format!("{} (Copy)", list.name);
                            self.pending_import_list = Some(list);
                            self.active_action = ListAction::Duplicate;
                            self.import_window_open = true;
                        }
                    }

                    if ui
                        .add_enabled(can_interact, egui::Button::new("🗑 Delete"))
                        .clicked()
                    {
                        self.delete_current_list();
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(can_interact, egui::Button::new("📤 Export"))
                        .clicked()
                    {
                        self.export_current_list();
                    }
                });
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_interact, egui::Button::new("➕ Add Mod"))
                    .clicked()
                {
                    self.search_window_open = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mods_to_download: Vec<String> = self
                        .filtered_mods
                        .iter()
                        .filter(|entry| {
                            !self.is_mod_loading(&entry.mod_id)
                                && self
                                    .download_status
                                    .get(&entry.mod_id)
                                    .map(|s| {
                                        matches!(s, DownloadStatus::Idle | DownloadStatus::Failed)
                                    })
                                    .unwrap_or(true)
                                && self.check_mod_compatibility(&entry.mod_id).unwrap_or(false)
                        })
                        .map(|e| e.mod_id.clone())
                        .collect();

                    let has_downloadable = !mods_to_download.is_empty();
                    if ui
                        .add_enabled(
                            can_interact && has_downloadable,
                            egui::Button::new("⬇ Download All"),
                        )
                        .clicked()
                    {
                        for mod_id in mods_to_download {
                            self.start_download(&mod_id);
                        }
                    }
                });
            });

            ui.separator();

            if let Some(list) = self.get_current_list() {
                if list.mods.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("No mods in this list");
                        ui.label("Click 'Add Mod' above to get started");
                    });
                }

                self.filter_mods();
                let mut delete_mod_id = None;

                let filtered_entries: Vec<ModEntry> = self.filtered_mods.clone();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in &filtered_entries {
                        let mod_id = &entry.mod_id;
                        self.load_mod_details_if_needed(mod_id);

                        let is_loading = self.is_mod_loading(mod_id);
                        let has_failed = self.mods_failed_loading.contains(mod_id);
                        let compatibility = self.check_mod_compatibility(mod_id);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(&entry.mod_name);
                                if let Some(mod_info) = self.get_mod_details(mod_id) {
                                    ui.label(format!(
                                        "v{} by {}",
                                        mod_info.version, mod_info.author
                                    ));
                                } else if is_loading {
                                    ui.label("⏳ Loading details...");
                                } else if has_failed {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        "⚠ Failed to load details",
                                    );
                                } else {
                                    ui.label("Details unavailable");
                                }
                                if let Some(false) = compatibility {
                                    ui.colored_label(egui::Color32::RED, "❌ Incompatible");
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let status = self
                                        .download_status
                                        .get(mod_id)
                                        .copied()
                                        .unwrap_or(DownloadStatus::Idle);

                                    match status {
                                        DownloadStatus::Idle => {
                                            let button = egui::Button::new("Download");
                                            let enabled = !is_loading
                                                && !has_failed
                                                && compatibility.unwrap_or(false);
                                            let mut response = ui.add_enabled(enabled, button);

                                            if is_loading {
                                                response = response.on_disabled_hover_text(
                                                    "Loading mod details...",
                                                );
                                            } else if has_failed {
                                                response = response.on_disabled_hover_text(
                                                    "Failed to load mod details",
                                                );
                                            }

                                            if response.clicked() {
                                                self.start_download(mod_id);
                                            }
                                        }
                                        DownloadStatus::Queued => {
                                            ui.add(
                                                egui::ProgressBar::new(0.0)
                                                    .text("Waiting...")
                                                    .desired_width(80.0),
                                            );
                                        }
                                        DownloadStatus::Downloading => {
                                            let progress = self
                                                .download_progress
                                                .get(mod_id)
                                                .copied()
                                                .unwrap_or(0.0);
                                            ui.add(
                                                egui::ProgressBar::new(progress)
                                                    .text(format!("{:.0}%", progress * 100.0))
                                                    .desired_width(80.0),
                                            );
                                        }
                                        DownloadStatus::Complete => {
                                            ui.label("✅");
                                        }
                                        DownloadStatus::Failed => {
                                            ui.label("❌");
                                        }
                                    }

                                    if ui.button("🗑").clicked() {
                                        delete_mod_id = Some(mod_id.clone());
                                    }
                                },
                            );
                        });
                        ui.separator();
                    }
                });

                if let Some(mod_id) = delete_mod_id {
                    self.delete_mod(&mod_id);
                }
            }
        });

        if self.search_window_open {
            if self.current_list_id.is_none() {
                self.search_window_open = false;
            } else {
                let overlay = egui::Area::new(egui::Id::new("search_overlay"))
                    .order(egui::Order::Background)
                    .fixed_pos(egui::pos2(0.0, 0.0));

                overlay.show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_black_alpha(128),
                    );

                    if ui
                        .interact(
                            screen_rect,
                            egui::Id::new("search_overlay_click"),
                            egui::Sense::click(),
                        )
                        .clicked()
                    {
                        self.search_window_open = false;
                    }
                });

                let mut open = self.search_window_open;
                egui::Window::new("Search Mods")
                    .collapsible(false)
                    .resizable(true)
                    .default_size([600.0, 400.0])
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.search_window_query)
                                    .hint_text("Search mod name or description...")
                                    .desired_width(400.0),
                            );
                            if ui.button("Search").clicked() {
                                self.perform_search();
                            }
                        });
                        ui.separator();

                        let mut mod_to_add = None;

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if self.search_window_results.is_empty() {
                                ui.label(if self.search_window_query.is_empty() {
                                    "Enter a search query"
                                } else {
                                    "No mods found"
                                });
                            } else {
                                for mod_info in &self.search_window_results {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(&mod_info.name);
                                            ui.add(
                                                egui::Label::new(&mod_info.description)
                                                    .wrap_mode(egui::TextWrapMode::Wrap),
                                            );
                                            ui.label(format!(
                                                "👤 {} | ⬇ {}",
                                                mod_info.author, mod_info.download_count
                                            ));
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("Add").clicked() {
                                                    mod_to_add = Some(mod_info.clone());
                                                }
                                            },
                                        );
                                    });
                                    ui.separator();
                                }
                            }
                        });

                        if let Some(mod_info) = mod_to_add {
                            self.add_mod_to_current_list(mod_info);
                        }
                    });
                self.search_window_open = open;
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
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
}
