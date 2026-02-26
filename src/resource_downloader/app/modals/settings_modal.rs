use crate::common::ui::ash_ui::AshUi;
use crate::common::ui::structs::modal_window::ModalWindow;
use crate::resource_downloader::business::xcache::CacheType;
use crate::resource_downloader::business::{Effect, SharedRDState};
use eframe::epaint::Color32;
use egui::{Id, Ui};
use std::path::PathBuf;

#[derive(Clone)]
pub struct SettingsModal {
    state: SharedRDState,
    save_on_close: bool,
    default_list_name: String,
    default_list_group_name: String,
    show_advanced_options: bool,
    cache_types: (bool, bool, bool, bool, bool, bool, bool, bool),
    show_import_warning: bool,
    pending_import_path: Option<PathBuf>,
}

impl SettingsModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state,
            save_on_close: false,
            default_list_name: String::new(),
            default_list_group_name: String::new(),
            show_advanced_options: false,
            cache_types: (false, false, false, false, false, false, false, false),
            show_import_warning: false,
            pending_import_path: None,
        }
    }
}

impl ModalWindow for SettingsModal {
    fn id(&self) -> Id {
        Id::new("settings")
    }

    fn title(&self) -> String {
        "Resource Manager Settings".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.label("Default list name:");
        ui.text_edit_singleline(&mut self.default_list_name);

        ui.label("Default list group name:");
        ui.text_edit_singleline(&mut self.default_list_group_name);

        ui.add_space(10.0);

        if ui.button("💾 Save").clicked() {
            self.save_on_close = true;
            *open = false;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        ui.strong("Backup & Restore");
        ui.weak("Export or import all settings and lists as a .flux-rm file");
        ui.add_space(5.0);

        let backup_progress = self.state.read().backup_progress.clone();
        if let Some((current, total, message)) = &backup_progress {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("{} ({}/{})", message, current, total));
            });
            ui.add_space(5.0);
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    backup_progress.is_none(),
                    egui::Button::new("📤 Export Backup"),
                )
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("flux-resource-manager-backup.flux-rm")
                    .add_filter("Flux RM Backup", &["flux-rm"])
                    .save_file()
                {
                    self.state.read().dispatch(Effect::ExportBackup { path });
                }
            }

            if ui
                .add_enabled(
                    backup_progress.is_none(),
                    egui::Button::new("📥 Import Backup"),
                )
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Flux RM Backup", &["flux-rm"])
                    .pick_file()
                {
                    self.pending_import_path = Some(path);
                    self.show_import_warning = true;
                }
            }
        });

        if self.show_import_warning {
            egui::Window::new("⚠ Warning: Import Backup")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_min_width(400.0);
                    ui.vertical(|ui| {
                        ui.colored_label(
                            Color32::from_rgb(255, 150, 0),
                            egui::RichText::new("⚠ Important Warning")
                                .strong()
                                .size(18.0),
                        );
                        ui.add_space(10.0);

                        ui.label(egui::RichText::new(
                            "Importing a backup will OVERWRITE all existing settings and lists!"
                        ).strong());
                        ui.add_space(5.0);

                        ui.label("This action will:");
                        ui.label("  • Replace all current lists with the backup");
                        ui.label("  • Replace all settings with the backup");
                        ui.label("  • Automatically reload all data");
                        ui.add_space(5.0);

                        ui.colored_label(
                            Color32::from_rgb(255, 100, 100),
                            egui::RichText::new(
                                "Make sure you have saved any unsaved changes before proceeding!",
                            )
                            .strong(),
                        );

                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new("✔ Yes, Import")
                                        .color(Color32::from_rgb(100, 255, 100)),
                                )
                                .clicked()
                            {
                                if let Some(path) = self.pending_import_path.take() {
                                    self.state.read().dispatch(Effect::ImportBackup { path });
                                }
                                self.show_import_warning = false;
                            }

                            if ui
                                .button(
                                    egui::RichText::new("❌ Cancel")
                                        .color(Color32::from_rgb(255, 100, 100)),
                                )
                                .clicked()
                            {
                                self.pending_import_path = None;
                                self.show_import_warning = false;
                            }
                        });
                    });
                });
        }

        ui.add_space(10.0);
        ui.separator();
        if ui
            .ash_expand_btn(format!(
                "{} Advanced",
                if self.show_advanced_options {
                    "🔽"
                } else {
                    "▶"
                }
            ))
            .clicked()
        {
            self.show_advanced_options = !self.show_advanced_options;
        }
        if self.show_advanced_options {
            ui.add_space(5.0);
            self.render_advanced_options(ui);
        }
        ui.add_space(5.0);
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
        let state = self.state.read();
        let config = state.config.read();
        self.default_list_name = config.default_list_name.clone();
        self.default_list_group_name = config.default_list_group_name.clone();
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }
        let state = self.state.write();
        {
            let mut config = state.config.write();
            config.default_list_name = self.default_list_name.clone();
            config.default_list_group_name = self.default_list_group_name.clone();
        }
        state.save_config();
    }
}

impl SettingsModal {
    fn render_advanced_options(&mut self, ui: &mut Ui) {
        ui.strong("Cache Management");
        ui.weak(
            "Cache contains search results, icons, versions, and more. Clearing \
         it can resolve loading issues but will require re-fetching data.",
        );
        ui.add_space(3.0);
        ui.checkbox(&mut self.cache_types.0, "Game Loader Cache");
        ui.checkbox(&mut self.cache_types.1, "Game Version Cache");
        ui.checkbox(&mut self.cache_types.2, "Slug Cache");
        ui.checkbox(&mut self.cache_types.3, "Metadata Cache");
        ui.checkbox(&mut self.cache_types.4, "Versions Cache");
        ui.checkbox(&mut self.cache_types.5, "Icons Cache");
        ui.checkbox(&mut self.cache_types.6, "Artifact Cache");
        ui.checkbox(&mut self.cache_types.7, "File Index Cache");
        ui.add_space(3.0);
        if self.cache_types.0 || self.cache_types.1 {
            ui.label("⚠ Clearing game version or loader cache requires restarting the app to take effect.");
            ui.add_space(3.0);
        }
        if ui
            .button(egui::RichText::new("🗑 Clear Cache").color(Color32::LIGHT_RED))
            .clicked()
        {
            {
                let mut to_clear_wo_mem = Vec::new();
                if self.cache_types.0 {
                    to_clear_wo_mem.push(CacheType::GameLoaders);
                }
                if self.cache_types.1 {
                    to_clear_wo_mem.push(CacheType::GameVersions);
                }
                self.state
                    .read()
                    .api()
                    .clear_core_cache(to_clear_wo_mem, false);
            }
            {
                let mut to_clear_in_mem = Vec::new();
                if self.cache_types.2 {
                    to_clear_in_mem.push(CacheType::ProjectSlug);
                }
                if self.cache_types.3 {
                    to_clear_in_mem.push(CacheType::ProjectMetadata);
                }
                if self.cache_types.4 {
                    to_clear_in_mem.push(CacheType::ProjectVersions);
                }
                if self.cache_types.5 {
                    to_clear_in_mem.push(CacheType::ProjectIcons);
                    self.state.read().api().icon_pool.clear_gpu_cache();
                }
                self.state
                    .read()
                    .api()
                    .clear_core_cache(to_clear_in_mem, true);
            }
            if self.cache_types.6 {
                self.state.read().api().artifact_cache.delete_all();
            }
            if self.cache_types.7 {
                self.state.read().dispatch(Effect::ClearFileIndexCache);
            }
        }
    }
}
