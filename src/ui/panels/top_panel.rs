use crate::app::{AppRuntime, Effect};
use crate::ui::ViewState;
use eframe::egui;
use eframe::epaint::Color32;
use egui::RichText;

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
                if tab_button(ui, "Minecraft Launcher", view_state.launcher_open).clicked() {
                    view_state.launcher_open = true;
                }

                if tab_button(ui, "/Resource Downloader", !view_state.launcher_open).clicked() {
                    view_state.launcher_open = false;
                }

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

fn tab_button(ui: &mut egui::Ui, text: &str, is_selected: bool) -> egui::Response {
    let bg_color = if is_selected {
        Color32::from_rgba_unmultiplied(0, 0, 0, 130)
    } else {
        Color32::TRANSPARENT
    };

    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    let button = egui::Button::new(RichText::new(text).heading())
        .fill(bg_color)
        .stroke(egui::Stroke::NONE)
        .corner_radius(0.0);

    let response = ui.add(button);

    if response.hovered() && !is_selected {
        ui.painter().rect_filled(
            response.rect,
            0.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 20),
        );
    }

    response
}
