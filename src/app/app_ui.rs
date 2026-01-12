use crate::app::AppState;
use eframe::egui;
use rfd::FileDialog;

use crate::app::*;
use crate::domain::{ModEntry, ModList};

pub struct App {
    state: AppState,
    list_search_query: String,
    search_query: String,
    selected_mod: Option<usize>,
    search_window_open: bool,
    search_window_query: String,
    rename_list_input: String,
    show_rename_input: bool,
    settings_window_open: bool,
    import_window_open: bool,
    import_name_input: String,
    active_action: ListAction,
    pending_import_list: Option<ModList>,
    sort_menu_open: bool,
    current_sort_mode: SortMode,
    current_filter_mode: FilterMode,
    current_order_mode: OrderMode,
    sort_field_rect: egui::Rect,
    show_archived: bool,
    list_settings_open: bool,
    list_settings_version: String,
    list_settings_loader: String,
    list_settings_dir: String,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let state = AppState::new(runtime);

        Self {
            state,
            list_search_query: String::new(),
            search_query: String::new(),
            selected_mod: None,
            search_window_open: false,
            search_window_query: String::new(),
            rename_list_input: String::new(),
            show_rename_input: false,
            settings_window_open: false,
            import_window_open: false,
            import_name_input: String::new(),
            active_action: ListAction::Import,
            pending_import_list: None,
            sort_menu_open: false,
            current_sort_mode: SortMode::Name,
            current_order_mode: OrderMode::Ascending,
            current_filter_mode: FilterMode::All,
            sort_field_rect: egui::Rect::NOTHING,
            show_archived: false,
            list_settings_open: false,
            list_settings_version: String::new(),
            list_settings_loader: String::new(),
            list_settings_dir: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.process_events();
        self.state.icon_service.update(ctx);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.settings_window_open = false;
            self.search_window_open = false;
            self.import_window_open = false;
            self.sort_menu_open = false;
            self.list_settings_open = false;
            self.pending_import_list = None;
        }

        if self.search_window_open && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.state.perform_search(&self.search_window_query);
        }

        if self.import_window_open && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.finalize_import_logic();
        }

        if self.sort_menu_open && ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                if !self.sort_field_rect.contains(pos) {
                    self.sort_menu_open = false;
                }
            }
        }

        self.draw_settings_window(ctx);
        self.draw_list_settings_window(ctx);
        self.draw_import_window(ctx);
        self.draw_legacy_window(ctx);
        self.draw_top_panel(ctx);
        self.draw_sidebar(ctx);
        self.draw_main_panel(ctx);
        self.draw_search_window(ctx);

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.persist_config_on_exit();
    }
}

impl App {
    fn draw_settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_window_open {
            return;
        }

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
        egui::Window::new("Default Settings")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Default Minecraft Version:");
                egui::ComboBox::from_id_salt("global_version_selector")
                    .selected_text(&self.state.selected_version)
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for version in &self.state.minecraft_versions {
                            if ui
                                .selectable_value(
                                    &mut self.state.selected_version,
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

                ui.add_space(10.0);

                ui.label("Default Mod Loader:");
                egui::ComboBox::from_id_salt("global_loader_selector")
                    .selected_text(&self.state.selected_loader)
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for loader in &self.state.mod_loaders {
                            if ui
                                .selectable_value(
                                    &mut self.state.selected_loader,
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

                ui.add_space(10.0);

                ui.label("Default Download Directory:");
                ui.horizontal(|ui| {
                    let mut dir = self.state.download_dir.clone();
                    ui.text_edit_singleline(&mut dir);
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .set_title("Select Default Download Directory")
                            .pick_folder()
                        {
                            self.state
                                .update_download_dir(path.to_string_lossy().to_string());
                        }
                    }
                });

                ui.add_space(10.0);
                ui.label("These settings apply to new lists and legacy imports by default.");
            });
        self.settings_window_open = open;
    }

    fn draw_list_settings_window(&mut self, ctx: &egui::Context) {
        if !self.list_settings_open {
            return;
        }

        if self.list_settings_version.is_empty() {
            if let Some(list) = self.state.get_current_list() {
                self.list_settings_version = if list.version.is_empty() {
                    "default".to_string()
                } else {
                    list.version.clone()
                };
                self.list_settings_loader = if list.loader.is_empty() {
                    "default".to_string()
                } else {
                    list.loader.clone()
                };
                self.list_settings_dir = if list.download_dir.is_empty() {
                    self.state.download_dir.clone()
                } else {
                    list.download_dir.clone()
                };
            }
        }

        let overlay = egui::Area::new(egui::Id::new("list_settings_overlay"))
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        let mut should_close = false;

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.screen_rect();
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

            if ui
                .interact(
                    screen_rect,
                    egui::Id::new("list_settings_overlay_click"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                should_close = true;
            }
        });

        let mut open = self.list_settings_open;

        egui::Window::new("List Settings")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Minecraft Version:");

                let display_version = if self.list_settings_version == "default" {
                    "Default".to_string()
                } else {
                    self.state
                        .minecraft_versions
                        .iter()
                        .find(|v| v.id == self.list_settings_version)
                        .map(|v| v.name.clone())
                        .unwrap_or_else(|| self.list_settings_version.clone())
                };

                let version_changed = egui::ComboBox::from_id_salt("list_version_selector")
                    .selected_text(display_version)
                    .show_ui(ui, |ui| {
                        let mut changed = false;

                        if ui
                            .selectable_value(
                                &mut self.list_settings_version,
                                "default".to_string(),
                                "Default",
                            )
                            .changed()
                        {
                            changed = true;
                        }

                        for version in &self.state.minecraft_versions {
                            if ui
                                .selectable_value(
                                    &mut self.list_settings_version,
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

                if version_changed {
                    self.state.update_list_settings(
                        self.list_settings_version.clone(),
                        self.list_settings_loader.clone(),
                        self.list_settings_dir.clone(),
                    );
                }

                ui.add_space(10.0);

                ui.label("Mod Loader:");

                let display_loader = if self.list_settings_loader == "default" {
                    "Default".to_string()
                } else {
                    self.state
                        .mod_loaders
                        .iter()
                        .find(|l| l.id == self.list_settings_loader)
                        .map(|l| l.name.clone())
                        .unwrap_or_else(|| self.list_settings_loader.clone())
                };

                let loader_changed = egui::ComboBox::from_id_salt("list_loader_selector")
                    .selected_text(display_loader)
                    .show_ui(ui, |ui| {
                        let mut changed = false;

                        if ui
                            .selectable_value(
                                &mut self.list_settings_loader,
                                "default".to_string(),
                                "Default",
                            )
                            .changed()
                        {
                            changed = true;
                        }

                        for loader in &self.state.mod_loaders {
                            if ui
                                .selectable_value(
                                    &mut self.list_settings_loader,
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

                if loader_changed {
                    self.state.update_list_settings(
                        self.list_settings_version.clone(),
                        self.list_settings_loader.clone(),
                        self.list_settings_dir.clone(),
                    );
                }

                ui.add_space(10.0);

                ui.label("Download Directory:");
                ui.horizontal(|ui| {
                    let dir_response = ui.text_edit_singleline(&mut self.list_settings_dir);
                    if dir_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.state.update_list_settings(
                            self.list_settings_version.clone(),
                            self.list_settings_loader.clone(),
                            self.list_settings_dir.clone(),
                        );
                    }

                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .set_title("Select List Download Directory")
                            .pick_folder()
                        {
                            self.list_settings_dir = path.to_string_lossy().to_string();
                            self.state.update_list_settings(
                                self.list_settings_version.clone(),
                                self.list_settings_loader.clone(),
                                self.list_settings_dir.clone(),
                            );
                        }
                    }
                });
            });

        if should_close {
            open = false;
        }

        let was_open = self.list_settings_open;
        self.list_settings_open = open;

        if was_open && !open {
            if !self.list_settings_dir.is_empty() {
                self.state.update_list_settings(
                    self.list_settings_version.clone(),
                    self.list_settings_loader.clone(),
                    self.list_settings_dir.clone(),
                );
            }
            self.list_settings_version.clear();
            self.list_settings_loader.clear();
            self.list_settings_dir.clear();
        }
    }

    fn draw_import_window(&mut self, ctx: &egui::Context) {
        if !(self.import_window_open && self.pending_import_list.is_some()) {
            return;
        }

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
            self.finalize_import_logic();
        }
    }

    fn finalize_import_logic(&mut self) {
        if let Some(mut list) = self.pending_import_list.take() {
            list.id = format!("list_{}", chrono::Utc::now().timestamp_millis());
            list.name = self.import_name_input.trim().to_string();

            if list.name.is_empty() {
                list.name = "Unnamed List".to_string();
            }

            self.state.finalize_import(list);

            self.import_window_open = false;
            self.import_name_input.clear();
        }
    }

    fn draw_legacy_window(&mut self, ctx: &egui::Context) {
        if self.state.legacy_state == LegacyState::Idle {
            return;
        }

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
                if matches!(self.state.legacy_state, LegacyState::Complete { .. }) {
                    is_open = false;
                }
            }
        });

        let mut should_import = false;

        let window_title = match &self.state.legacy_state {
            LegacyState::InProgress { .. } => "Processing...",
            _ => "Operation Complete",
        };

        let mut suggested_name0 = String::new();

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.set_min_width(300.0);

                match &self.state.legacy_state {
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
                        suggested_name,
                        successful,
                        failed,
                        warnings,
                        is_import,
                    } => {
                        let success_count = successful.len();
                        let fail_count = failed.len();
                        let warn_count = warnings.len();
                        let is_importable = self.state.pending_legacy_mods.is_some();

                        suggested_name0 = suggested_name.clone();

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
            if let Some(mods) = self.state.pending_legacy_mods.take() {
                let entries = mods
                    .into_iter()
                    .map(|m| ModEntry {
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        added_at: chrono::Utc::now(),
                        archived: false,
                    })
                    .collect();

                self.pending_import_list = Some(ModList {
                    id: format!("list_{}", chrono::Utc::now().timestamp()),
                    name: suggested_name0.clone(),
                    created_at: chrono::Utc::now(),
                    mods: entries,
                    version: String::new(),
                    loader: String::new(),
                    download_dir: String::new(),
                });
                self.import_name_input = suggested_name0.clone();
                self.active_action = ListAction::Import;
                self.import_window_open = true;
            }
            self.state.legacy_state = LegacyState::Idle;
        } else if !is_open {
            self.state.legacy_state = LegacyState::Idle;
            self.state.pending_legacy_mods = None;
        }
    }

    fn draw_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Minecraft Mod Downloader");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Default Settings").clicked() {
                        self.settings_window_open = true;
                    }

                    if ui
                        .button("↻ Reload All")
                        .on_hover_text("Refresh all mod details")
                        .clicked()
                    {
                        self.state.reload_all_mods();
                    }
                });
            });
        });
    }
    fn draw_sidebar(&mut self, ctx: &egui::Context) {
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
                    self.state.create_new_list();
                }

                if ui
                    .add_sized([25.0, 25.0], egui::Button::new("📥"))
                    .on_hover_text("Import")
                    .clicked()
                {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Mod List", &["toml"])
                        .add_filter("Legacy Mod List", &["mods", "all-mods", "queue-mods"])
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
                                self.state.start_legacy_import(path);
                            }
                            _ => {}
                        }
                    }
                }
            });

            ui.add_space(4.0);
            ui.separator();

            let filtered_lists: Vec<_> = self
                .state
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
                    let selected = self.state.current_list_id.as_ref() == Some(&list.id);
                    let display_text = if list.version.is_empty() && list.loader.is_empty() {
                        format!("{} ({})", list.name, list.mods.len())
                    } else {
                        format!(
                            "{} [{} | {}] ({})",
                            list.name,
                            list.version,
                            list.loader,
                            list.mods.len()
                        )
                    };

                    if ui.selectable_label(selected, display_text).clicked() {
                        if selected {
                            self.state.current_list_id = None;
                        } else {
                            self.state.current_list_id = Some(list.id.clone());
                        }
                        self.selected_mod = None;
                    }
                }
            });
        });
    }

    fn draw_main_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.current_list_id.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("No list selected");
                    ui.label("Select a list from the sidebar or create a new one");
                });
                return;
            }

            let can_interact = self.state.current_list_id.is_some();

            ui.horizontal(|ui| {
                if let Some(list) = self.state.get_current_list() {
                    ui.heading(format!("{}", &list.name));
                    ui.add_space(4.0);

                    ui.add_space(5.0);

                    let ver = self.state.get_effective_version();
                    let loader = self.state.get_effective_loader();
                    let ver_name = self
                        .state
                        .minecraft_versions
                        .iter()
                        .find(|v| v.id == ver)
                        .map(|v| v.name.as_str())
                        .unwrap_or(&ver);

                    let loader_name = self
                        .state
                        .mod_loaders
                        .iter()
                        .find(|l| l.id == loader)
                        .map(|l| l.name.as_str())
                        .unwrap_or(&loader);

                    ui.label(
                        egui::RichText::new(format!("{} | {}", ver_name, loader_name))
                            .small()
                            .weak(),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.show_rename_input {
                        ui.text_edit_singleline(&mut self.rename_list_input);
                        if ui.button("✔").clicked() {
                            let new_name = self.rename_list_input.clone();
                            let list_to_save = if let Some(list) = self.state.get_current_list_mut()
                            {
                                list.name = new_name;
                                Some(list.clone())
                            } else {
                                None
                            };

                            if let Some(list) = list_to_save {
                                self.state.save_list(&list);
                            }
                            self.show_rename_input = false;
                        }
                        if ui.button("❌").clicked() {
                            self.show_rename_input = false;
                        }
                    } else {
                        if ui
                            .add_enabled(can_interact, egui::Button::new("🗑 Delete"))
                            .clicked()
                        {
                            self.state.delete_current_list();
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("✏ Rename"))
                            .clicked()
                        {
                            self.show_rename_input = true;
                            if let Some(list) = self.state.get_current_list() {
                                self.rename_list_input = list.name.clone();
                            }
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("👥 Duplicate"))
                            .clicked()
                        {
                            if let Some(list) = self.state.get_current_list().cloned() {
                                self.import_name_input = format!("{} (Copy)", list.name);
                                self.pending_import_list = Some(list);
                                self.active_action = ListAction::Duplicate;
                                self.import_window_open = true;
                            }
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("📤 Export"))
                            .clicked()
                        {
                            if let Some(list) = self.state.get_current_list() {
                                if let Some(save_path) = FileDialog::new()
                                    .add_filter("Mod List", &["toml"])
                                    .add_filter(
                                        "Legacy Mod List",
                                        &["mods", "all-mods", "queue-mods"],
                                    )
                                    .set_title("Export Mod List")
                                    .set_file_name(&format!("{}.toml", list.name))
                                    .save_file()
                                {
                                    self.state.export_current_list(save_path);
                                }
                            }
                        }

                        let sort_label = match self.current_order_mode {
                            OrderMode::Ascending => "⬇ Sort",
                            OrderMode::Descending => "⬆ Sort",
                        };
                        let sort_btn = ui
                            .add_enabled(can_interact, egui::Button::new(sort_label))
                            .on_hover_text("Sort and Filter");
                        if sort_btn.clicked() {
                            self.sort_menu_open = !self.sort_menu_open;
                        }
                        self.sort_field_rect = sort_btn.rect;

                        if ui.button("⚙ List Settings").clicked() {
                            self.list_settings_open = true;
                            self.list_settings_version.clear();
                        }
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
                    let filtered_mods = self.state.get_filtered_mods(
                        &self.search_query,
                        self.current_sort_mode,
                        self.current_order_mode,
                        self.current_filter_mode,
                    );

                    let mut existing_count = 0;
                    let mut missing_ids: Vec<String> = Vec::new();

                    for entry in &filtered_mods {
                        let is_downloading = self.state.mods_being_loaded.contains(&entry.mod_id);
                        let status = self.state.download_status.get(&entry.mod_id);
                        let is_queued_or_active = status
                            .map(|s| {
                                matches!(s, DownloadStatus::Queued | DownloadStatus::Downloading)
                            })
                            .unwrap_or(false);

                        if is_downloading || is_queued_or_active {
                            continue;
                        }

                        if let Some(info) = self.state.get_mod_details(&entry.mod_id) {
                            if self.state.is_mod_file_present(&*info) {
                                existing_count += 1;
                            } else {
                                if self
                                    .state
                                    .check_mod_compatibility(&entry.mod_id)
                                    .unwrap_or(false)
                                {
                                    missing_ids.push(entry.mod_id.clone());
                                }
                            }
                        }
                    }

                    let mods_to_download: Vec<String> = filtered_mods
                        .iter()
                        .filter(|entry| {
                            !self.state.mods_being_loaded.contains(&entry.mod_id)
                                && self
                                    .state
                                    .download_status
                                    .get(&entry.mod_id)
                                    .map(|s| {
                                        matches!(s, DownloadStatus::Idle | DownloadStatus::Failed)
                                    })
                                    .unwrap_or(true)
                                && self
                                    .state
                                    .check_mod_compatibility(&entry.mod_id)
                                    .unwrap_or(false)
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
                            self.state.start_download(&mod_id);
                        }
                    }

                    let show_missing_button = existing_count > 0 && !missing_ids.is_empty();
                    if show_missing_button {
                        ui.add_space(5.0);
                        if ui
                            .add_enabled(can_interact, egui::Button::new("⬇ Download Missing"))
                            .clicked()
                        {
                            for mod_id in missing_ids {
                                self.state.start_download(&mod_id);
                            }
                        }
                    }
                });
            });

            ui.separator();

            if let Some(list) = self.state.get_current_list() {
                if list.mods.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("No mods in this list");
                        ui.label("Click 'Add Mod' above to get started");
                    });
                }

                let filtered_entries = self.state.get_filtered_mods(
                    &self.search_query,
                    self.current_sort_mode,
                    self.current_order_mode,
                    self.current_filter_mode,
                );

                let active_mods: Vec<_> = filtered_entries.iter().filter(|e| !e.archived).collect();
                let archived_mods: Vec<_> =
                    filtered_entries.iter().filter(|e| e.archived).collect();

                let mut toggle_archive_id = None;
                let mut delete_mod_id = None;

                let total_existing_in_list: usize = active_mods
                    .iter()
                    .filter(|e| {
                        if let Some(info) = self.state.get_mod_details(&e.mod_id) {
                            self.state.is_mod_file_present(&*info)
                        } else {
                            false
                        }
                    })
                    .count();

                let highlight_missing = total_existing_in_list > 0;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut render_mod_entry = |ui: &mut egui::Ui, entry: &ModEntry| {
                        let mod_id = &entry.mod_id;
                        self.state.load_mod_details_if_needed(mod_id);

                        let is_loading = self.state.mods_being_loaded.contains(mod_id);
                        let has_failed = self.state.mods_failed_loading.contains(mod_id);
                        let compatibility = self.state.check_mod_compatibility(mod_id);
                        let is_archived = entry.archived;

                        let is_present = if let Some(mod_info) = self.state.get_mod_details(mod_id)
                        {
                            self.state.is_mod_file_present(&*mod_info)
                        } else {
                            false
                        };

                        ui.horizontal(|ui| {
                            if let Some(mod_info) = self.state.get_mod_details(mod_id)
                                && !mod_info.icon_url.is_empty()
                                && let Some(handle) =
                                    self.state.icon_service.get(&mod_info.icon_url)
                            {
                                ui.add(
                                    egui::Image::from_texture(handle)
                                        .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                                );
                            } else {
                                ui.add_sized(egui::vec2(32.0, 32.0), egui::Spinner::new());
                            }
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                let mut name_text = egui::RichText::new(&entry.mod_name);
                                if is_archived {
                                    name_text = name_text.weak();
                                }
                                ui.hyperlink_to(
                                    name_text,
                                    format!("https://modrinth.com/project/{}", entry.mod_id),
                                );
                                if let Some(mod_info) = self.state.get_mod_details(mod_id) {
                                    ui.label(format!(
                                        "v{} by {}",
                                        mod_info.version, mod_info.author
                                    ));
                                } else if is_loading {
                                    ui.label("⏳ Loading details...");
                                } else if has_failed {
                                    if ui
                                        .button(
                                            egui::RichText::new("⚠ Failed to load details")
                                                .color(egui::Color32::YELLOW),
                                        )
                                        .on_hover_text("Reloads this mods details")
                                        .clicked()
                                    {
                                        self.state.force_reload_mod(mod_id);
                                    }
                                } else {
                                    ui.label("Details unavailable");
                                }
                                if let Some(false) = compatibility {
                                    ui.colored_label(egui::Color32::RED, "❌ Incompatible");
                                } else if highlight_missing
                                    && !is_present
                                    && !is_archived
                                    && !is_loading
                                {
                                    ui.colored_label(egui::Color32::LIGHT_YELLOW, "⚠ Missing");
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let status = self
                                        .state
                                        .download_status
                                        .get(mod_id)
                                        .copied()
                                        .unwrap_or(DownloadStatus::Idle);

                                    match status {
                                        DownloadStatus::Idle => {
                                            if is_present {
                                                ui.label("✅");
                                            }
                                            if !is_archived {
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
                                                    self.state.start_download(mod_id);
                                                }
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
                                                .state
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

                                    let archive_text = if is_archived {
                                        "📂 Unarchive"
                                    } else {
                                        "📁"
                                    };
                                    if ui.button(archive_text).clicked() {
                                        toggle_archive_id = Some(mod_id.clone());
                                    }

                                    if ui.button("🗑").clicked() {
                                        delete_mod_id = Some(mod_id.clone());
                                    }
                                },
                            );
                        });
                        ui.separator();
                    };

                    if !active_mods.is_empty() {
                        for entry in active_mods {
                            render_mod_entry(ui, entry);
                        }
                    }

                    if !archived_mods.is_empty() {
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let icon = if self.show_archived { "🔽" } else { "▶" };
                            if ui
                                .button(format!("{} Archived ({})", icon, archived_mods.len()))
                                .clicked()
                            {
                                self.show_archived = !self.show_archived;
                            }
                        });

                        if self.show_archived {
                            ui.add_space(4.0);
                            for entry in archived_mods {
                                render_mod_entry(ui, entry);
                            }
                        }
                    }
                });

                if let Some(mod_id) = delete_mod_id {
                    self.state.delete_mod(&mod_id);
                }
                if let Some(mod_id) = toggle_archive_id {
                    self.state.toggle_archive_mod(&mod_id);
                }
            }
        });

        if self.sort_menu_open {
            let popup_pos = self.sort_field_rect.left_bottom() + egui::vec2(0.0, 5.0);

            egui::Area::new(egui::Id::new("sort_menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(popup_pos)
                .show(ctx, |ui| {
                    let popup = egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(140.0);
                        ui.vertical(|ui| {
                            ui.label("Sort by:");
                            if ui
                                .selectable_value(
                                    &mut self.current_sort_mode,
                                    SortMode::Name,
                                    "Name",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.current_sort_mode,
                                    SortMode::DateAdded,
                                    "Date Added",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.current_sort_mode,
                                    SortMode::Compatibility,
                                    "Compatibility",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }

                            ui.separator();

                            ui.label("Order:");
                            if ui
                                .selectable_value(
                                    &mut self.current_order_mode,
                                    OrderMode::Ascending,
                                    "⬇ Ascending",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.current_order_mode,
                                    OrderMode::Descending,
                                    "⬆ Descending",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }

                            ui.separator();

                            ui.label("Filter:");
                            if ui
                                .selectable_value(
                                    &mut self.current_filter_mode,
                                    FilterMode::All,
                                    "❔ None",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.current_filter_mode,
                                    FilterMode::CompatibleOnly,
                                    "✅ Compatible Only",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.current_filter_mode,
                                    FilterMode::IncompatibleOnly,
                                    "❌ Incompatible Only",
                                )
                                .clicked()
                            {
                                self.sort_menu_open = false;
                            }
                        });
                    });
                    self.sort_field_rect = popup.response.rect;
                });
        }
    }

    fn draw_search_window(&mut self, ctx: &egui::Context) {
        if self.search_window_open {
            if self.state.current_list_id.is_none() {
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
                            ui.checkbox(
                                &mut self.state.search_filter_exact,
                                "Match version/loader",
                            );
                            if ui.button("Search").clicked() {
                                self.state.perform_search(&self.search_window_query);
                            }
                        });
                        ui.separator();

                        let mut mod_to_add = None;

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if self.state.search_window_results.is_empty() {
                                ui.label(if self.search_window_query.is_empty() {
                                    "Enter a search query"
                                } else {
                                    "No mods found"
                                });
                            } else {
                                for mod_info in &self.state.search_window_results {
                                    ui.horizontal(|ui| {
                                        if !mod_info.icon_url.is_empty()
                                            && let Some(handle) =
                                                self.state.icon_service.get(&mod_info.icon_url)
                                        {
                                            ui.add(
                                                egui::Image::from_texture(handle)
                                                    .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                                            );
                                        } else {
                                            ui.add_sized(
                                                egui::vec2(32.0, 32.0),
                                                egui::Spinner::new(),
                                            );
                                        }
                                        ui.add_space(4.0);
                                        ui.vertical(|ui| {
                                            ui.hyperlink_to(
                                                &mod_info.name,
                                                format!(
                                                    "https://modrinth.com/project/{}",
                                                    mod_info.id
                                                ),
                                            );
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
                            self.state.add_mod_to_current_list(mod_info);
                            self.search_window_open = false;
                            self.search_window_query.clear();
                        }
                    });
                self.search_window_open = open;
            }
        }
    }
}
