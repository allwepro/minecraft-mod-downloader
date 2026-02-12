use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::folder_actions::FolderActions;
use crate::resource_downloader::domain::FolderLnk;
use egui::{Color32, Id, Ui};

#[allow(dead_code)]
#[derive(Clone)]
pub struct FolderSettingsModal {
    state: SharedRDState,
    folder_lnk: FolderLnk,
    folder_name: String,
    original_name: String,
    save_on_close: bool,
}

#[allow(dead_code)]
impl FolderSettingsModal {
    pub fn new(state: SharedRDState, folder_lnk: FolderLnk, folder_name: String) -> Self {
        Self {
            state,
            folder_lnk,
            folder_name: folder_name.clone(),
            original_name: folder_name,
            save_on_close: false,
        }
    }
}

impl ModalWindow for FolderSettingsModal {
    fn id(&self) -> Id {
        Id::new("folder_settings").with(self.folder_lnk.id())
    }

    fn title(&self) -> String {
        format!("📁 Folder Settings: {}", self.original_name)
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.add_space(8.0);

        // Folder Name Section
        egui::Frame::default()
            .fill(ui.visuals().faint_bg_color)
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(60)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.heading("Folder Name");
                ui.add_space(8.0);

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.folder_name)
                        .desired_width(ui.available_width())
                        .hint_text("Enter folder name..."),
                );

                if response.changed() && self.folder_name.starts_with(' ') {
                    self.folder_name = self.folder_name.trim_start().to_string();
                }

                let name_changed = self.folder_name != self.original_name;
                if name_changed {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("⚠ Name has been changed")
                            .small()
                            .color(Color32::from_rgb(255, 200, 100)),
                    );
                }
            });

        ui.add_space(12.0);

        // Folder Info Section
        egui::Frame::default()
            .fill(ui.visuals().faint_bg_color)
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(60)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.heading("Folder Information");
                ui.add_space(8.0);

                let list_count = {
                    let state = self.state.read();
                    let config = state.config.read();
                    config
                        .folder_assignments
                        .values()
                        .filter(|fid| *fid == self.folder_lnk.id())
                        .count()
                };

                ui.horizontal(|ui| {
                    ui.label("📋 Lists in folder:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(list_count.to_string())
                                .strong()
                                .color(Color32::from_rgb(100, 200, 255)),
                        );
                    });
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("🆔 Folder ID:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(self.folder_lnk.id()).small().weak());
                    });
                });
            });

        ui.add_space(12.0);

        egui::Frame::default()
            .fill(ui.visuals().faint_bg_color)
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(60)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.heading("Actions");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("👥  Duplicate Folder").clicked() {
                        FolderActions::duplicate_folder(
                            self.state.clone(),
                            self.folder_lnk.clone(),
                        );
                        *open = false;
                    }

                    if ui
                        .button("🗑  Delete Folder")
                        .on_hover_text("This will not delete the lists inside")
                        .clicked()
                    {
                        FolderActions::delete_folder(self.state.clone(), self.folder_lnk.clone());
                        *open = false;
                    }
                });
            });

        ui.add_space(12.0);

        ui.separator();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let can_save = !self.folder_name.trim().is_empty();

            if ui
                .add_enabled(can_save, egui::Button::new("💾 Save"))
                .clicked()
            {
                self.save_on_close = true;
                *open = false;
            }

            if ui.button("❌ Cancel").clicked() {
                *open = false;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Reset").clicked() {
                    self.folder_name = self.original_name.clone();
                }
            });
        });
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
        // Reload original name in case it was changed elsewhere
        let state = self.state.read();
        let config = state.config.read();
        if let Some(folder) = config.folders.iter().find(|f| f.id == self.folder_lnk.id()) {
            self.folder_name = folder.name.clone();
            self.original_name = folder.name.clone();
        }
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }

        let name = self.folder_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        if name != self.original_name {
            FolderActions::rename_folder(self.state.clone(), self.folder_lnk.clone(), name);
        }
    }
}
