use crate::app::{AppState, Effect, ListAction};
use crate::domain::{ModEntry, ModList, ModLoader, ProjectType};
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

        if let Some(collection) = state.pending_collection.take() {
            view_state.collection_import_loading = false;
            view_state.collection_import_window_open = false;
            view_state.reset_collection_import();

            let mods: Vec<ModEntry> = collection
                .projects
                .into_iter()
                .map(|(project_id, project_name)| ModEntry {
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
                download_dir: String::new(),
                content_type: ProjectType::Mod,
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
            effects.push(Effect::ImportModrinthCollection { collection_id });
        }

        view_state.collection_import_window_open = is_open || view_state.collection_import_loading;
        effects
    }
}
