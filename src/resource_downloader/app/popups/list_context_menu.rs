use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::folder_actions::FolderActions;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::domain::{FolderLnk, ListLnk};
use eframe::egui;
use egui::{Color32, Id, Ui};

#[derive(Clone)]
pub struct ListContextMenu {
    state: SharedRDState,
    list_lnk: ListLnk,
}

impl ListContextMenu {
    pub fn new(state: SharedRDState, list_lnk: ListLnk) -> Self {
        Self { state, list_lnk }
    }
}

impl Popup for ListContextMenu {
    fn id(&self) -> Id {
        Id::new("list_context_menu").with(&self.list_lnk)
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        let list_lnk = self.list_lnk.clone();

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            let (current_folder_id, folders) = {
                let state = self.state.read();
                let config = state.config.read();
                let current = config
                    .folder_assignments
                    .get(&list_lnk.to_string())
                    .cloned();
                (current, config.folders.clone())
            };

            if !folders.is_empty() {
                ui.menu_button("📁  Move to Folder", |ui| {
                    ui.set_min_width(180.0);

                    let is_in_no_folder = current_folder_id.is_none();
                    let no_folder_text = if is_in_no_folder {
                        egui::RichText::new("✓ No Folder").strong()
                    } else {
                        egui::RichText::new("   No Folder")
                    };

                    if ui.button(no_folder_text).clicked() {
                        FolderActions::move_list_to_folder(
                            self.state.clone(),
                            list_lnk.to_string(),
                            None,
                        );
                        *open = false;
                    }

                    ui.separator();

                    for folder in folders {
                        let is_current = current_folder_id.as_ref() == Some(&folder.id);

                        let folder_text = if is_current {
                            egui::RichText::new(format!("✓ {}", folder.name)).strong()
                        } else {
                            egui::RichText::new(format!("   {}", folder.name))
                        };

                        let mut button = egui::Button::new(folder_text);

                        if is_current {
                            button =
                                button.fill(Color32::from_rgba_unmultiplied(100, 150, 200, 50));
                        }

                        if ui.add(button).clicked() {
                            if !is_current {
                                FolderActions::move_list_to_folder(
                                    self.state.clone(),
                                    list_lnk.to_string(),
                                    Some(FolderLnk::new(folder.id)),
                                );
                            }
                            *open = false;
                        }
                    }
                });

                ui.separator();
            }

            if ui.button("📂  Open Folder").clicked() {
                ListActions::open_folder(self.state.clone(), list_lnk.clone());
                *open = false;
            }

            if ui.button("👥  Duplicate").clicked() {
                ListActions::duplicate_list(self.state.clone(), list_lnk.clone());
                *open = false;
            }

            ui.separator();

            if let Some(folder_id) = &current_folder_id {
                let folder_name = {
                    let state = self.state.read();
                    let config = state.config.read();
                    config
                        .folders
                        .iter()
                        .find(|f| &f.id == folder_id)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string())
                };

                if ui
                    .button(format!("📤  Remove from '{}'", folder_name))
                    .clicked()
                {
                    FolderActions::move_list_to_folder(
                        self.state.clone(),
                        list_lnk.to_string(),
                        None,
                    );
                    *open = false;
                }
            }

            let delete_btn = egui::Button::new(
                egui::RichText::new("🗑  Delete").color(Color32::from_rgb(255, 100, 100)),
            );

            if ui.add(delete_btn).clicked() {
                ListActions::delete_list(self.state.clone(), list_lnk.clone());
                *open = false;
            }
        });
    }
}
