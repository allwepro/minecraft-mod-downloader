use crate::app::{AppState, Effect, ListAction};
use crate::domain::{ModEntry, ModList, ProjectType};
use crate::infra::ConfigManager;
use crate::ui::ViewState;
use chrono::Utc;
use eframe::egui;

pub struct CollectionImportWindow;

impl CollectionImportWindow {
    fn parse_collection_id(input: &str) -> Option<String> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        if input.contains("modrinth.com/collection/") {
            input
                .split("/collection/")
                .last()
                .map(|s| s.split(['/', '?', '#']).next().unwrap_or(s).to_string())
                .filter(|s| !s.is_empty())
        } else {
            Some(input.to_string())
        }
    }

    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        let available_types = if state.pending_collection.is_some() {
            view_state.collection_import_loading = false;
            if !view_state.collection_import_finalizing {
                view_state.collection_import_finalizing = true;
                let collection = state.pending_collection.as_ref().unwrap();
                let types: Vec<ProjectType> = collection
                    .project_type_suggestions
                    .keys()
                    .copied()
                    .collect();
                if !types.is_empty() {
                    view_state.collection_import_selected_type = types[0];
                }
            }
            let collection = state.pending_collection.as_ref().unwrap();
            collection
                .project_type_suggestions
                .keys()
                .copied()
                .collect::<Vec<_>>()
        } else {
            vec![ProjectType::Mod]
        };

        if let Some(error) = state.collection_import_error.take() {
            view_state.collection_import_loading = false;
            view_state.collection_import_error = Some(error);
        }

        let overlay_id = egui::Id::new("collection_import_overlay");
        let overlay = egui::Area::new(overlay_id)
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.content_rect();
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

            if ui
                .interact(screen_rect, overlay_id.with("click"), egui::Sense::click())
                .clicked()
                && !view_state.collection_import_loading
            {
                view_state.collection_import_window_open = false;
                view_state.reset_collection_import();
            }
        });

        let mut should_import = false;
        let mut is_open = view_state.collection_import_window_open;

        egui::Window::new("Import Modrinth Collection")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.set_min_width(400.0);

                ui.label("Enter a Modrinth Collection URL or ID:");
                ui.add_space(4.0);

                let input_enabled = !view_state.collection_import_loading || !view_state.collection_import_finalizing;
                ui.add_enabled(
                    input_enabled,
                    egui::TextEdit::singleline(&mut view_state.collection_import_input)
                        .hint_text("https://modrinth.com/collection/ZCxg7r1U")
                        .desired_width(ui.available_width()),
                );

                if view_state.collection_import_finalizing {
                    if available_types.is_empty() {
                        ui.add_space(8.0);
                        ui.label("Collection contains no supported project types.");
                    } else {
                        if available_types.len() > 1 {
                            ui.add_space(8.0);
                            ui.label("⚠ Collection contains multiple content types. Lists can contain only one content type. To import other types, please import them separately.");
                        }
                        ui.add_space(12.0);

                        ui.label("Content Type:");
                        egui::ComboBox::from_id_salt("collection_import_type_selector")
                            .selected_text(view_state.collection_import_selected_type.display_name())
                            .show_ui(ui, |ui| {
                                for p_type in available_types.iter() {
                                    if ui
                                        .selectable_value(
                                            &mut view_state.collection_import_selected_type,
                                            *p_type,
                                            p_type.display_name(),
                                        )
                                        .changed()
                                    {
                                        effects.extend(
                                            state.ensure_loaders_for_type(view_state.collection_import_selected_type),
                                        );
                                    }
                                }
                            });
                    }
                }


                if let Some(ref error) = view_state.collection_import_error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::RED, format!("❌ {error}"));
                }

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if view_state.collection_import_loading {
                        ui.add(egui::Spinner::new());
                        ui.label("Loading collection...");
                    } else {
                        let can_import =
                            Self::parse_collection_id(&view_state.collection_import_input)
                                .is_some();

                        if ui
                            .add_enabled(can_import, egui::Button::new("Import"))
                            .clicked()
                        {
                            should_import = true;
                        }
                    }
                });
            });

        if !view_state.collection_import_loading
            && !view_state.collection_import_finalizing
            && ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && Self::parse_collection_id(&view_state.collection_import_input).is_some()
        {
            should_import = true;
        }

        if should_import
            && view_state.collection_import_finalizing
            && let Some(collection) = state.pending_collection.take()
        {
            let content_type = view_state.collection_import_selected_type;

            view_state.collection_import_finalizing = false;
            view_state.collection_import_window_open = false;
            view_state.reset_collection_import();

            let filtered_projects: Vec<(String, String, ProjectType)> = collection
                .projects
                .into_iter()
                .filter(|(_, _, project_type)| *project_type == content_type)
                .collect();

            let download_dir = if let Some(dir) =
                ConfigManager::get_default_minecraft_download_dir(content_type)
            {
                dir.to_string_lossy().to_string()
            } else {
                String::new()
            };

            let mods: Vec<ModEntry> = filtered_projects
                .into_iter()
                .map(|(project_id, project_name, _)| ModEntry {
                    mod_id: project_id,
                    mod_name: project_name,
                    added_at: Utc::now(),
                    archived: false,
                    compatibility_override: false,
                })
                .collect();

            let pending_list = ModList {
                id: String::new(), // Will be set on finalize
                name: collection.name.clone(),
                created_at: Utc::now(),
                mods,
                version: collection
                    .project_type_suggestions
                    .get(&content_type)
                    .map(|(ver, _)| ver.clone())
                    .unwrap(),
                loader: collection
                    .project_type_suggestions
                    .get(&content_type)
                    .map(|(_, loader)| loader.clone())
                    .unwrap(),
                download_dir,
                content_type,
            };

            view_state.import_name_input = collection.name;
            view_state.pending_import_list = Some(pending_list);
            view_state.active_action = ListAction::Import;
            view_state.import_window_open = true;

            return effects;
        }

        if !is_open && !view_state.collection_import_loading {
            view_state.collection_import_window_open = false;
            view_state.reset_collection_import();
        } else if should_import
            && !view_state.collection_import_finalizing
            && let Some(collection_id) =
                Self::parse_collection_id(&view_state.collection_import_input)
        {
            view_state.collection_import_loading = true;
            view_state.collection_import_error = None;
            effects.push(Effect::ImportModrinthCollection { collection_id });
        }

        view_state.collection_import_window_open = is_open || view_state.collection_import_loading;
        effects
    }
}
