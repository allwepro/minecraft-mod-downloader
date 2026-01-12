use crate::app::{AppState, Effect};
use crate::domain::{ModEntry, ModList, ProjectType};
use crate::ui::{ViewState, dialogs::Dialogs};
use eframe::egui;

pub struct LegacyImportSettingsWindow;

impl LegacyImportSettingsWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        if view_state.legacy_import_version.is_empty() {
            view_state.legacy_import_version = state.get_effective_version();
            view_state.legacy_import_loader = state.get_effective_loader();
            view_state.legacy_import_dir = state.get_effective_download_dir();
        }

        let overlay_id = egui::Id::new("legacy_import_settings_overlay");
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
                view_state.legacy_import_settings_open = false;
                view_state.reset_legacy_import();
            }
        });

        let mut is_open = view_state.legacy_import_settings_open;
        let mut should_close = false;

        egui::Window::new("📥 Import Mod List")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.label("List Name:");
                ui.text_edit_singleline(&mut view_state.legacy_import_name);

                ui.add_space(10.0);

                ui.label("Minecraft Version:");
                let display_version = state
                    .minecraft_versions
                    .iter()
                    .find(|v| v.id == view_state.legacy_import_version)
                    .map(|v| v.name.clone())
                    .unwrap_or_else(|| view_state.legacy_import_version.clone());

                egui::ComboBox::from_id_salt("legacy_import_version_selector")
                    .selected_text(display_version)
                    .show_ui(ui, |ui| {
                        for version in &state.minecraft_versions {
                            ui.selectable_value(
                                &mut view_state.legacy_import_version,
                                version.id.clone(),
                                &version.name,
                            );
                        }
                    });

                ui.add_space(10.0);

                ui.label("Loader:");
                let loaders = state.mod_loaders.clone();
                let display_loader = loaders
                    .iter()
                    .find(|l| l.id == view_state.legacy_import_loader)
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| view_state.legacy_import_loader.clone());

                egui::ComboBox::from_id_salt("legacy_import_loader_selector")
                    .selected_text(display_loader)
                    .show_ui(ui, |ui| {
                        for loader in &loaders {
                            ui.selectable_value(
                                &mut view_state.legacy_import_loader,
                                loader.id.clone(),
                                &loader.name,
                            );
                        }
                    });

                ui.add_space(10.0);

                ui.label("Download Directory:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut view_state.legacy_import_dir);
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = Dialogs::pick_folder() {
                            view_state.legacy_import_dir = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        if let Some(mods) = view_state.legacy_import_mods.take() {
                            let entries = mods
                                .into_iter()
                                .map(|m| ModEntry {
                                    mod_id: m.id.clone(),
                                    mod_name: m.name.clone(),
                                    added_at: chrono::Utc::now(),
                                    archived: false,
                                })
                                .collect();

                            let list = ModList {
                                id: format!("list_{}", chrono::Utc::now().timestamp()),
                                name: view_state.legacy_import_name.clone(),
                                created_at: chrono::Utc::now(),
                                mods: entries,
                                version: view_state.legacy_import_version.clone(),
                                loader: state
                                    .mod_loaders
                                    .iter()
                                    .find(|l| l.id == view_state.legacy_import_loader)
                                    .cloned()
                                    .unwrap_or(crate::domain::ModLoader {
                                        id: view_state.legacy_import_loader.clone(),
                                        name: view_state.legacy_import_loader.clone(),
                                    }),
                                download_dir: view_state.legacy_import_dir.clone(),
                                content_type: ProjectType::Mod,
                            };

                            state.mod_lists.push(list.clone());
                            state.current_list_id = Some(list.id.clone());
                            effects.push(Effect::SaveList { list });
                        }
                        should_close = true;
                    }
                });
            });

        if should_close {
            view_state.reset_legacy_import();
            view_state.legacy_import_settings_open = false;
        } else {
            view_state.legacy_import_settings_open = is_open;
        }
        effects
    }
}
