use crate::app::{AppState, Effect, ListAction};
use crate::ui::ViewState;
use eframe::egui;

pub struct ImportWindow;

impl ImportWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        if view_state.pending_import_list.is_none() {
            view_state.import_window_open = false;
            return effects;
        }

        let overlay_id = egui::Id::new("import_overlay");
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
            {
                view_state.import_window_open = false;
                view_state.pending_import_list = None;
            }
        });

        let mod_count = view_state
            .pending_import_list
            .as_ref()
            .map(|l| l.mods.len())
            .unwrap_or(0);

        let title = match view_state.active_action {
            ListAction::Import => "📥 Import Mod List",
            ListAction::Duplicate => "👥 Duplicate Mod List",
        };

        let mut should_finalize = false;
        let mut should_close = false;
        let mut is_open = view_state.import_window_open;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.label("List Name:");
                ui.text_edit_singleline(&mut view_state.import_name_input);

                ui.add_space(8.0);
                ui.label(egui::RichText::new(format!("Contains {} items", mod_count)).weak());

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

        if should_close || !is_open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            view_state.import_window_open = false;
            view_state.pending_import_list = None;
        } else if should_finalize || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(mut list) = view_state.pending_import_list.take() {
                list.id = format!("list_{}", chrono::Utc::now().timestamp_millis());
                list.name = view_state.import_name_input.trim().to_string();

                if list.name.is_empty() {
                    list.name = "Unnamed List".to_string();
                }

                effects.extend(state.finalize_import(list));
                view_state.import_window_open = false;
                view_state.import_name_input.clear();
            }
        }

        view_state.import_window_open = is_open;
        effects
    }
}
