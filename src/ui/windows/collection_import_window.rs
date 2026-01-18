use crate::app::{AppState, Effect, ListAction};
use crate::domain::{ModEntry, ModList, ModLoader, ProjectType};
use crate::ui::ViewState;
use chrono::Utc;
use eframe::egui;

fn get_download_dir_for_type(state: &AppState, project_type: ProjectType) -> String {
    let base_dir = if state.current_list_id.is_some() {
        state.get_effective_download_dir()
    } else {
        String::new()
    };

    if base_dir.is_empty() {
        return String::new();
    }

    match project_type {
        ProjectType::Mod => {
            format!("{}/mods", base_dir)
        }
        ProjectType::ResourcePack => {
            format!("{}/resourcepacks", base_dir)
        }
        ProjectType::Shader => {
            format!("{}/shaderpacks", base_dir)
        }
        ProjectType::Datapack => {
            format!("{}/datapacks", base_dir)
        }
        ProjectType::Modpack => {
            // Modpacks usually go to the base directory or a modpacks folder
            base_dir
        }
        ProjectType::Plugin => {
            format!("{}/plugins", base_dir)
        }
    }
}

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

        if let Some(collection) = state.pending_collection.take() {
            view_state.collection_import_loading = false;
            view_state.collection_import_window_open = false;
            view_state.reset_collection_import();

            // Projects are already filtered by the API based on selected_type
            let filtered_projects: Vec<(String, String, ProjectType)> =
                collection.projects.into_iter().collect();

            let (primary_content_type, download_dir) = if filtered_projects.is_empty() {
                (ProjectType::Mod, String::new())
            } else {
                // Since all projects are of the same type (filtered by API), use that type
                let content_type = filtered_projects
                    .first()
                    .map(|(_, _, pt)| *pt)
                    .unwrap_or(ProjectType::Mod);
                let dir = get_download_dir_for_type(state, content_type);
                (content_type, dir)
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
                version: collection.recommended_version.clone(),
                loader: ModLoader {
                    id: collection.recommended_loader.clone(),
                    name: collection.recommended_loader.clone(),
                },
                download_dir,
                content_type: primary_content_type,
            };

            view_state.import_name_input = collection.name;
            view_state.pending_import_list = Some(pending_list);
            view_state.active_action = ListAction::Import;
            view_state.import_window_open = true;

            return effects;
        }

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
        let mut should_close = false;
        let mut is_open = view_state.collection_import_window_open;

        egui::Window::new("📦 Import Modrinth Collection")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.set_min_width(400.0);

                ui.label("Enter a Modrinth Collection URL or ID:");
                ui.add_space(4.0);

                let input_enabled = !view_state.collection_import_loading;
                ui.add_enabled(
                    input_enabled,
                    egui::TextEdit::singleline(&mut view_state.collection_import_input)
                        .hint_text("https://modrinth.com/collection/... or collection ID")
                        .desired_width(ui.available_width()),
                );

                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(
                        "Example: https://modrinth.com/collection/abc123 or abc123",
                    )
                    .small()
                    .weak(),
                );

                ui.add_space(12.0);

                ui.label("Select content type to import:");
                ui.add_space(4.0);

                let content_types = vec![
                    (ProjectType::Mod, "🔧 Mods"),
                    (ProjectType::ResourcePack, "🎨 Resourcepacks"),
                    (ProjectType::Shader, "✨ Shader"),
                    (ProjectType::Datapack, "📦 Datapacks"),
                    (ProjectType::Modpack, "📚 Modpacks"),
                    (ProjectType::Plugin, "⚙️ Plugins"),
                ];

                let selected_text = content_types
                    .iter()
                    .find(|(pt, _)| *pt == view_state.collection_import_selected_type)
                    .map(|(_, label)| label)
                    .unwrap_or(&"Mods");

                egui::ComboBox::from_id_salt("collection_import_type")
                    .selected_text(*selected_text)
                    .show_ui(ui, |ui| {
                        for (content_type, label) in content_types {
                            ui.selectable_value(
                                &mut view_state.collection_import_selected_type,
                                content_type,
                                label,
                            );
                        }
                    });

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
                            .add_enabled(can_import, egui::Button::new("📥 Import"))
                            .clicked()
                        {
                            should_import = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    }
                });
            });

        if !view_state.collection_import_loading
            && ctx.input(|i| i.key_pressed(egui::Key::Enter))
            && Self::parse_collection_id(&view_state.collection_import_input).is_some()
        {
            should_import = true;
        }

        if should_close || (!is_open && !view_state.collection_import_loading) {
            view_state.collection_import_window_open = false;
            view_state.reset_collection_import();
        } else if should_import
            && let Some(collection_id) =
                Self::parse_collection_id(&view_state.collection_import_input)
        {
            view_state.collection_import_loading = true;
            view_state.collection_import_error = None;
            effects.push(Effect::ImportModrinthCollection {
                collection_id,
                selected_type: view_state.collection_import_selected_type,
            });
        }

        view_state.collection_import_window_open = is_open || view_state.collection_import_loading;
        effects
    }
}
