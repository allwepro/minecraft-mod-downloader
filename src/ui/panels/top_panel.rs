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

                ui.add_space(20.0);

                // Tab switching
                ui.style_mut().spacing.item_spacing.x = 0.0;

                let launcher_btn =
                    ui.selectable_label(view_state.launcher_open, "🚀 Minecraft Launcher");
                if launcher_btn.clicked() {
                    view_state.launcher_open = true;
                }

                let downloader_btn =
                    ui.selectable_label(!view_state.launcher_open, "📦 Resource Downloader");
                if downloader_btn.clicked() {
                    view_state.launcher_open = false;
                }

                ui.style_mut().spacing.item_spacing.x = 8.0; // Reset spacing

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        view_state.settings_window_open = true;
                    }
                });
            });
        });

        effects
    }
}
