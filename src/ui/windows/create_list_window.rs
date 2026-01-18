use crate::app::{AppRuntime, AppState, Effect};
use crate::domain::ProjectType;
use crate::infra::ConfigManager;
use crate::ui::{ViewState, dialogs::Dialogs};
use eframe::egui;

pub struct CreateListWindow;

impl CreateListWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
        _runtime: &mut AppRuntime,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        if view_state.new_list_name.is_empty() {
            view_state.new_list_name = state.default_list_name.clone();
        }
        if view_state.new_list_version.is_empty() && !state.minecraft_versions.is_empty() {
            view_state.new_list_version = state.minecraft_versions[0].id.clone();
        }
        if view_state.new_list_loader.is_empty()
            && let Some(loaders) = state.loaders_for_type(view_state.new_list_type)
            && !loaders.is_empty()
        {
            view_state.new_list_loader = loaders[0].id.clone();
        }
        if (view_state.new_list_dir.is_empty() || !view_state.new_list_dir_edited)
            && let Some(default_dir) =
                ConfigManager::get_default_minecraft_download_dir(view_state.new_list_type)
        {
            view_state.new_list_dir = default_dir.to_string_lossy().to_string();
            view_state.new_list_dir_edited = false;
        }

        let overlay = egui::Area::new(egui::Id::new("create_list_overlay"))
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.content_rect();
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

            if ui
                .interact(
                    screen_rect,
                    egui::Id::new("create_list_overlay_click"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                view_state.create_list_window_open = false;
            }
        });

        let mut is_open = view_state.create_list_window_open;
        let mut should_close = false;

        egui::Window::new("➕ Create New List")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.label("List Name:");
                ui.text_edit_singleline(&mut view_state.new_list_name);

                ui.add_space(10.0);

                ui.label("Content Type:");
                egui::ComboBox::from_id_salt("new_list_type_selector")
                    .selected_text(view_state.new_list_type.display_name())
                    .show_ui(ui, |ui| {
                        for p_type in &[
                            ProjectType::Mod,
                            ProjectType::ResourcePack,
                            ProjectType::Shader,
                            ProjectType::Datapack,
                            ProjectType::Plugin,
                        ] {
                            if ui
                                .selectable_value(
                                    &mut view_state.new_list_type,
                                    *p_type,
                                    p_type.display_name(),
                                )
                                .changed()
                            {
                                effects.extend(
                                    state.ensure_loaders_for_type(view_state.new_list_type),
                                );

                                view_state.new_list_loader.clear();

                                if let Some(loaders) =
                                    state.loaders_for_type(view_state.new_list_type)
                                    && !loaders.is_empty()
                                {
                                    view_state.new_list_loader = loaders[0].id.clone();
                                }
                            }
                        }
                    });

                ui.add_space(10.0);

                ui.label("Minecraft Version:");
                let display_version = state
                    .minecraft_versions
                    .iter()
                    .find(|v| v.id == view_state.new_list_version)
                    .map(|v| v.name.clone())
                    .unwrap_or_else(|| view_state.new_list_version.clone());

                egui::ComboBox::from_id_salt("new_list_version_selector")
                    .selected_text(display_version)
                    .show_ui(ui, |ui| {
                        for version in &state.minecraft_versions {
                            ui.selectable_value(
                                &mut view_state.new_list_version,
                                version.id.clone(),
                                &version.name,
                            );
                        }
                    });

                ui.add_space(10.0);

                ui.label("Loader:");
                let loaders_opt = state.loaders_for_type(view_state.new_list_type);
                let is_loading = state.is_loading_loaders_for_type(view_state.new_list_type);

                match loaders_opt {
                    Some(loaders) if !loaders.is_empty() => {
                        let loaders_vec: Vec<crate::domain::ModLoader> = loaders.to_vec();

                        let is_current_valid = !view_state.new_list_loader.is_empty()
                            && loaders_vec
                                .iter()
                                .any(|l| l.id == view_state.new_list_loader);

                        if !is_current_valid {
                            view_state.new_list_loader = loaders_vec[0].id.clone();
                        }

                        let display_loader = loaders_vec
                            .iter()
                            .find(|l| l.id == view_state.new_list_loader)
                            .map(|l| l.name.clone())
                            .unwrap_or_else(|| loaders_vec[0].name.clone());

                        egui::ComboBox::from_id_salt("new_list_loader_selector")
                            .selected_text(display_loader)
                            .show_ui(ui, |ui| {
                                for loader in &loaders_vec {
                                    ui.selectable_value(
                                        &mut view_state.new_list_loader,
                                        loader.id.clone(),
                                        &loader.name,
                                    );
                                }
                            });
                    }
                    _ => {
                        let text = if is_loading {
                            "Loading..."
                        } else {
                            "Not available"
                        };
                        egui::ComboBox::from_id_salt("new_list_loader_selector")
                            .selected_text(text)
                            .show_ui(ui, |_ui| {});

                        if is_loading {
                            ui.label(egui::RichText::new("Loading loaders...").weak());
                        }
                    }
                }

                ui.add_space(10.0);

                ui.label("Download Directory:");
                ui.horizontal(|ui| {
                    if ui
                        .text_edit_singleline(&mut view_state.new_list_dir)
                        .changed()
                    {
                        view_state.new_list_dir_edited = true;
                    };
                    if ui.button("Browse...").clicked()
                        && let Some(path) =
                            Dialogs::pick_minecraft_mods_folder(view_state.new_list_type)
                    {
                        view_state.new_list_dir_edited = true;
                        view_state.new_list_dir = path.to_string_lossy().to_string();
                    }
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        state.create_new_list(
                            view_state.new_list_name.trim().to_string(),
                            view_state.new_list_type,
                            view_state.new_list_version.clone(),
                            view_state.new_list_loader.clone(),
                            view_state.new_list_dir.clone(),
                        );
                        view_state.reset_create_list();
                        should_close = true;
                    }
                });
            });

        if should_close {
            view_state.create_list_window_open = false;
        } else {
            view_state.create_list_window_open = is_open;
        }
        effects
    }
}
