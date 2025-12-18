use crate::adapters::ModrinthProvider;
use crate::domain::*;
use crate::infra::ConfigManager;
use chrono::Utc;
use eframe::egui;
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

pub struct App {
    provider: Arc<dyn ModProvider>,
    config_manager: Arc<ConfigManager>,
    minecraft_versions: Vec<MinecraftVersion>,
    mod_loaders: Vec<ModLoader>,
    selected_version: String,
    selected_loader: String,
    previous_version: String,
    previous_loader: String,
    mod_lists: Vec<ModList>,
    current_list_id: String,
    filtered_mods: Vec<ModEntry>,
    search_query: String,
    selected_mod: Option<usize>,
    download_progress: HashMap<String, f32>,
    download_status: HashMap<String, DownloadStatus>,
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
    search_window_open: bool,
    search_window_query: String,
    search_window_results: Vec<ModInfo>,
    rename_list_input: String,
    show_rename_input: bool,
    settings_window_open: bool,
    mod_cache: Arc<Mutex<ModCache>>,
    mods_being_loaded: HashSet<String>,
    mods_failed_loading: HashSet<String>,
    download_dir: String,
    _runtime: tokio::runtime::Runtime,
    runtime_handle: tokio::runtime::Handle,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let runtime_handle = runtime.handle().clone();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(100);
        let (event_tx, event_rx) = mpsc::channel::<Event>(100);

        let provider: Arc<dyn ModProvider> = Arc::new(ModrinthProvider::new());
        let provider_clone = provider.clone();

        runtime_handle.spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let provider = provider_clone.clone();
                let event_tx = event_tx.clone();

                tokio::spawn(async move {
                    match cmd {
                        Command::SearchMods {
                            query,
                            version,
                            loader,
                        } => {
                            if let Ok(results) =
                                provider.search_mods(&query, &version, &loader).await
                            {
                                let _ = event_tx.send(Event::SearchResults(results)).await;
                            }
                        }
                        Command::FetchModDetails { mod_id, version, loader } => {
                            match provider.fetch_mod_details(&mod_id, &version, &loader).await {
                                Ok(details) => {
                                    let _ = event_tx.send(Event::ModDetails(details)).await;
                                }
                                Err(e) => {
                                    log::warn!("Failed to fetch mod details for {}: {}", mod_id, e);
                                    let _ = event_tx
                                        .send(Event::ModDetailsFailed { mod_id })
                                        .await;
                                }
                            }
                        }
                        Command::DownloadMod {
                            mod_info,
                            download_dir,
                        } => {
                            let mod_id = mod_info.id.clone();
                            let filename = format!("{}.jar", mod_info.name.replace(" ", "_"));
                            let destination = std::path::Path::new(&download_dir).join(&filename);

                            let event_tx_progress = event_tx.clone();
                            let mod_id_clone = mod_id.clone();

                            let result = provider
                                .download_mod(
                                    &mod_info.download_url,
                                    &destination,
                                    Box::new(move |progress| {
                                        let _ = event_tx_progress.try_send(
                                            Event::DownloadProgress {
                                                mod_id: mod_id_clone.clone(),
                                                progress,
                                            },
                                        );
                                    }),
                                )
                                .await;

                            let _ = event_tx
                                .send(Event::DownloadComplete {
                                    mod_id,
                                    success: result.is_ok(),
                                })
                                .await;
                        }
                    }
                });
            }
        });

        let config_manager = Arc::new(
            ConfigManager::new().expect("Failed to create config manager"),
        );

        let (selected_version, selected_loader, download_dir, mut mod_lists, current_list_id, minecraft_versions, mod_loaders) = {
            let cm = config_manager.clone();
            let prov = provider.clone();

            runtime_handle.block_on(async {
                let _ = cm.ensure_dirs().await;

                let config = if cm.config_exists() {
                    cm.load_config().await.unwrap_or_else(|_| {
                        runtime_handle.block_on(cm.create_default_config()).unwrap()
                    })
                } else {
                    cm.create_default_config().await.unwrap()
                };

                let mut lists = match cm.load_all_lists().await {
                    Ok(lists) => {
                        if lists.is_empty() {
                            let default_list = ModList {
                                id: "main".to_string(),
                                name: "My Mods".to_string(),
                                created_at: Utc::now(),
                                mods: Vec::new(),
                            };
                            let _ = cm.save_list(&default_list).await;
                            vec![default_list]
                        } else {
                            lists
                        }
                    }
                    Err(_) => {
                        let default_list = ModList {
                            id: "main".to_string(),
                            name: "My Mods".to_string(),
                            created_at: Utc::now(),
                            mods: Vec::new(),
                        };
                        vec![default_list]
                    }
                };

                let current_list_id = if lists.iter().any(|l| l.id == config.current_list_id) {
                    config.current_list_id.clone()
                } else {
                    lists.first().unwrap().id.clone()
                };

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

        let mod_cache = Arc::new(Mutex::new(ModCache::new(500, 1)));

        Self {
            provider,
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
            mod_cache,
            mods_being_loaded: HashSet::new(),
            mods_failed_loading: HashSet::new(),
            download_dir,
            _runtime: runtime,
            runtime_handle,
        }
    }

    fn invalidate_and_reload(&mut self) {
        self.mod_cache.blocking_lock().cache.clear();
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
        self.mod_lists.iter().find(|l| l.id == self.current_list_id)
    }

    fn get_current_list_mut(&mut self) -> Option<&mut ModList> {
        self.mod_lists
            .iter_mut()
            .find(|l| l.id == self.current_list_id)
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
        }
    }

    fn get_mod_details(&self, mod_id: &str) -> Option<ModInfo> {
        self.mod_cache.blocking_lock().get(mod_id)
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

        if self.mod_cache.blocking_lock().contains_valid(mod_id) {
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
            .insert(mod_id.to_string(), DownloadStatus::Downloading);
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
                    self.mod_cache.blocking_lock().insert(mod_id.clone(), mod_info);
                    self.mods_being_loaded.remove(&mod_id);
                }
                Event::ModDetailsFailed { mod_id } => {
                    self.mods_being_loaded.remove(&mod_id);
                    self.mods_failed_loading.insert(mod_id);
                }
                Event::DownloadProgress { mod_id, progress } => {
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

    fn add_mod_to_current_list(&mut self, mod_info: ModInfo) {
        if let Some(current_list) = self.get_current_list_mut() {
            if !current_list.mods.iter().any(|e| e.mod_id == mod_info.id) {
                current_list.mods.push(ModEntry {
                    mod_id: mod_info.id.clone(),
                    mod_name: mod_info.name.clone(),
                    added_at: Utc::now(),
                });
                self.download_status
                    .insert(mod_info.id.clone(), DownloadStatus::Idle);
                self.mod_cache
                    .blocking_lock()
                    .insert(mod_info.id.clone(), mod_info);
            }
        }
        self.search_window_open = false;
        self.search_window_query.clear();
        self.search_window_results.clear();
    }

    fn delete_current_list(&mut self) {
        if self.mod_lists.len() > 1 {
            let list_id = self.current_list_id.clone();
            self.mod_lists.retain(|l| l.id != list_id);

            if let Some(first_list) = self.mod_lists.first() {
                self.current_list_id = first_list.id.clone();
            }

            self.selected_mod = None;

            let cm = self.config_manager.clone();
            self.runtime_handle.spawn(async move {
                let _ = cm.delete_list(&list_id).await;
            });
        }
    }

    fn share_current_list(&mut self) {
        if let Some(current_list) = self.get_current_list() {
            let list_id = current_list.id.clone();
            let list_name = current_list.name.clone();

            if let Some(save_path) = FileDialog::new()
                .set_title("Export Mod List")
                .set_file_name(&format!("{}.toml", list_name))
                .save_file()
            {
                let cm = self.config_manager.clone();
                self.runtime_handle.spawn(async move {
                    let source_path = cm.get_lists_dir().join(format!("{}.toml", list_id));
                    let _ = tokio::fs::copy(&source_path, &save_path).await;
                });
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
        }

        if self.search_window_open && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.perform_search();
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
                            if ui.selectable_value(
                                &mut self.selected_version,
                                version.id.clone(),
                                &version.name,
                            ).changed() {
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
                            if ui.selectable_value(
                                &mut self.selected_loader,
                                loader.id.clone(),
                                &loader.name,
                            ).changed() {
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

                if ui.button("⚙ Settings").clicked() {
                    self.settings_window_open = true;
                }
            });
        });

        egui::SidePanel::left("sidebar").show(ctx, |ui| {
            ui.heading("Mod Lists");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for list in &self.mod_lists {
                    let selected = self.current_list_id == list.id;
                    if ui
                        .selectable_label(selected, format!("{} ({})", list.name, list.mods.len()))
                        .clicked()
                    {
                        self.current_list_id = list.id.clone();
                        self.selected_mod = None;
                    }
                }
            });

            ui.separator();

            if ui.button("➕ New List").clicked() {
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

                self.mod_lists.push(new_list);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("⬇ Download All").clicked() {
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
                        })
                        .map(|e| e.mod_id.clone())
                        .collect();

                    for mod_id in mods_to_download {
                        self.start_download(&mod_id);
                    }
                }

                if ui.button("📤 Share").clicked() {
                    self.share_current_list();
                }

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
                    if ui.button("✏ Rename").clicked() {
                        self.show_rename_input = true;
                        if let Some(list) = self.get_current_list() {
                            self.rename_list_input = list.name.clone();
                        }
                    }
                    if ui.button("🗑 Delete").clicked() {
                        self.delete_current_list();
                    }
                }
            });
            ui.separator();

            if let Some(list) = self.get_current_list() {
                if list.mods.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("No mods in this list");
                        ui.label("Click '+' below to add mods");
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
                                            let enabled = !is_loading && !has_failed && compatibility.unwrap_or(false);
                                            let mut response =
                                                ui.add_enabled(enabled, button);

                                            if is_loading {
                                                response = response
                                                    .on_disabled_hover_text(
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

                    if ui.selectable_label(false, "➕ Add Mod...").clicked() {
                        self.search_window_open = true;
                    }
                });

                if let Some(mod_id) = delete_mod_id {
                    self.delete_mod(&mod_id);
                }
            }
        });

        if self.search_window_open {
            let overlay = egui::Area::new(egui::Id::new("search_overlay"))
                .order(egui::Order::Background)
                .fixed_pos(egui::pos2(0.0, 0.0));

            overlay.show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

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
        }
    }
}