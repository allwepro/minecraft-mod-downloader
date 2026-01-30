use crate::common::app::App;
use crate::common::app::Tab::{Launcher, ResourceDownloader};
use crate::common::app_icon::show_app_icon;
use crate::common::prefabs::view_controller::ViewController;
use eframe::egui;
use eframe::epaint::Color32;
use egui::RichText;

pub struct TopPanel;

impl TopPanel {
    pub fn show(ctx: &egui::Context, app: &mut App) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                show_app_icon(ui, 25.0);
                ui.add_space(3.0);
                ui.heading("Flux");
                ui.add_space(2.0);
                if tab_button(ui, "Launcher", app.open_tab == Launcher).clicked() {
                    app.open_tab = Launcher;
                }
                ui.add_space(2.0);
                ui.heading("&");
                ui.add_space(2.0);
                if tab_button(
                    ui,
                    "Resource Downloader",
                    app.open_tab == ResourceDownloader,
                )
                .clicked()
                {
                    app.open_tab = ResourceDownloader;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let actions = match app.open_tab {
                        ResourceDownloader => app.rd_manager.get_top_bar_actions(),
                        Launcher => vec![], //app.launcher_manager.get_top_bar_actions()
                    };

                    for action in actions {
                        let btn = ui.button(&action.label).on_hover_text(&action.tooltip);
                        if btn.clicked() {
                            (action.callback)(ctx);
                        }
                    }
                });
            });
        });
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

pub struct TopBarAction {
    pub label: String,
    pub tooltip: String,
    pub callback: Box<dyn FnOnce(&egui::Context)>,
}

impl TopBarAction {
    pub fn new(label: &str, tooltip: &str, cb: impl FnOnce(&egui::Context) + 'static) -> Self {
        Self {
            label: label.to_string(),
            tooltip: tooltip.to_string(),
            callback: Box::new(cb),
        }
    }
}
