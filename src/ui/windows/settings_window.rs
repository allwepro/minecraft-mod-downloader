use crate::app::{AppState, Effect};
use crate::ui::ViewState;
use eframe::egui;

pub struct SettingsWindow;

impl SettingsWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        if view_state.app_settings_default_name.is_empty() && view_state.settings_window_open {
            view_state.app_settings_default_name = state.default_list_name.clone();
        }

        let overlay = egui::Area::new(egui::Id::new("settings_overlay"))
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.content_rect();
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
                view_state.settings_window_open = false;
                view_state.app_settings_default_name.clear();
            }
        });

        let mut is_open = view_state.settings_window_open;
        let mut should_close = false;

        egui::Window::new("⚙ Settings")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(400.0)
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.heading("Application Settings");
                ui.separator();

                ui.label("Default list name:");
                ui.text_edit_singleline(&mut view_state.app_settings_default_name);

                ui.add_space(10.0);

                if ui.button("💾 Save Settings").clicked() {
                    state.default_list_name = view_state.app_settings_default_name.clone();
                    effects.push(Effect::SaveConfig {
                        current_list_id: state.current_list_id.clone(),
                        default_list_name: state.default_list_name.clone(),
                    });
                    should_close = true;
                }
            });

        if should_close {
            view_state.settings_window_open = false;
            view_state.app_settings_default_name.clear();
        } else {
            view_state.settings_window_open = is_open;
        }

        if !view_state.settings_window_open {
            view_state.app_settings_default_name.clear();
        }

        effects
    }
}
