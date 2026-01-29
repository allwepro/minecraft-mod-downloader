use crate::app::{AppRuntime, Effect};
use crate::ui::ViewState;
use eframe::egui;

pub struct TopPanel;

impl TopPanel {
    pub fn show(
        ctx: &egui::Context,
        view_state: &mut ViewState,
        _runtime: &mut AppRuntime,
    ) -> Vec<Effect> {
        let effects = Vec::new();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Minecraft Mod Downloader");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        view_state.settings_window_open = true;
                    }

                    /*if let Some(list_id) = &state.current_list_id {
                        if let Some(current_list) = state.get_list_by_id(list_id) {
                            ui.separator();
                            ui.label(format!("📋 {}", current_list.name));
                        }
                    }*/
                });
            });
        });

        effects
    }
}
