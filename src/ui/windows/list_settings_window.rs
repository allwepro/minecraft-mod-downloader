use crate::app::{AppRuntime, AppState, Effect};
use crate::ui::{ViewState, dialogs::Dialogs};
use eframe::egui;

pub struct ListSettingsWindow;

impl ListSettingsWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
        _runtime: &mut AppRuntime,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        if let Some(list) = state.get_current_list()
            && view_state.list_settings_version.is_empty()
        {
            view_state.list_settings_version = list.version.clone();
            view_state.list_settings_loader = list.loader.id.clone();
            view_state.list_settings_dir = if list.download_dir.is_empty() {
                state.get_effective_download_dir()
            } else {
                list.download_dir.clone()
            };
        }

        let overlay = egui::Area::new(egui::Id::new("list_settings_overlay"))
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.content_rect();
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
                view_state.list_settings_open = false;
            }
        });

        let mut is_open = view_state.list_settings_open;
        let mut should_close = false;

        egui::Window::new("📋 List Settings")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(450.0)
            .open(&mut is_open)
            .show(ctx, |ui| {
                if let Some(list_id) = &state.current_list_id
                    && let Some(list) = state.get_list_by_id(list_id)
                {
                    let content_type = list.content_type;
                    let list_name = list.name.clone();
                    let list_version = list.version.clone();
                    let list_loader_name = list.loader.name.clone();
                    let list_loader_id = list.loader.id.clone();
                    let list_download_dir = list.download_dir.clone();
                    let list_clone = list.clone();

                    ui.heading(&list_name);
                    ui.separator();

                    ui.label("Minecraft Version:");
                    egui::ComboBox::from_id_salt("list_settings_version")
                        .selected_text(if view_state.list_settings_version.is_empty() {
                            &list_version
                        } else {
                            &view_state.list_settings_version
                        })
                        .show_ui(ui, |ui| {
                            for ver in &state.minecraft_versions {
                                if ui.selectable_label(false, &ver.name).clicked() {
                                    view_state.list_settings_version = ver.id.clone();
                                }
                            }
                        });

                    ui.add_space(5.0);

                    ui.label("Mod Loader:");

                    let loader_effects = state.ensure_loaders_for_type(content_type);
                    effects.extend(loader_effects);

                    let loaders = state.loaders_for_type(content_type).unwrap_or(&[]);

                    let selected_loader_name = if view_state.list_settings_loader.is_empty() {
                        list_loader_name.clone()
                    } else {
                        loaders
                            .iter()
                            .find(|l| l.id == view_state.list_settings_loader)
                            .map(|l| l.name.clone())
                            .unwrap_or_else(|| view_state.list_settings_loader.clone())
                    };

                    egui::ComboBox::from_id_salt("list_settings_loader")
                        .selected_text(&selected_loader_name)
                        .show_ui(ui, |ui| {
                            for loader in loaders.iter() {
                                if ui.selectable_label(false, &loader.name).clicked() {
                                    view_state.list_settings_loader = loader.id.clone();
                                }
                            }
                        });

                    ui.add_space(5.0);

                    ui.label("Download Directory:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut view_state.list_settings_dir);
                        if ui.button("📁 Browse").clicked()
                            && let Some(path) = Dialogs::pick_folder()
                        {
                            view_state.list_settings_dir = path.display().to_string();
                        }
                    });

                    ui.add_space(10.0);

                    if ui.button("💾 Save Settings").clicked() {
                        let new_version = if view_state.list_settings_version.is_empty() {
                            list_version
                        } else {
                            view_state.list_settings_version.clone()
                        };

                        let new_loader_id = if view_state.list_settings_loader.is_empty() {
                            list_loader_id
                        } else {
                            view_state.list_settings_loader.clone()
                        };

                        let new_dir = if view_state.list_settings_dir.is_empty() {
                            list_download_dir
                        } else {
                            view_state.list_settings_dir.clone()
                        };

                        let loader_obj = state
                            .mod_loaders
                            .iter()
                            .find(|l| l.id == new_loader_id)
                            .cloned()
                            .unwrap_or(crate::domain::ModLoader {
                                id: new_loader_id.clone(),
                                name: new_loader_id.clone(),
                            });

                        let mut updated_list = list_clone;
                        updated_list.version = new_version;
                        updated_list.loader = loader_obj;
                        updated_list.download_dir = new_dir;

                        if let Some(pos) =
                            state.mod_lists.iter().position(|l| l.id == updated_list.id)
                        {
                            state.mod_lists[pos] = updated_list.clone();
                        }

                        state.effective_settings_cache.clear();
                        effects.extend(state.invalidate_and_reload());

                        effects.push(Effect::SaveList { list: updated_list });

                        should_close = true;
                    }
                }
            });

        if should_close {
            view_state.reset_list_settings();
            view_state.list_settings_open = false;
        } else {
            view_state.list_settings_open = is_open;
        }
        effects
    }
}
