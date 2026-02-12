use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::modals::create_folder_modal::CreateFolderModal;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::folder_actions::FolderActions;
use crate::resource_downloader::domain::FolderLnk;
use eframe::egui;
use egui::{Color32, Id, Ui};

#[derive(Clone)]
pub struct FolderContextMenu {
    state: SharedRDState,
    folder_lnk: FolderLnk,
    folder_name: String,
}

impl FolderContextMenu {
    pub fn new(state: SharedRDState, folder_lnk: FolderLnk, folder_name: String) -> Self {
        Self {
            state,
            folder_lnk,
            folder_name,
        }
    }
}

impl Popup for FolderContextMenu {
    fn id(&self) -> Id {
        Id::new("folder_context_menu").with(&self.folder_lnk)
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        let folder_lnk = self.folder_lnk.clone();

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            if ui.button("📁  Create Subfolder").clicked() {
                let modal =
                    CreateFolderModal::with_parent(self.state.clone(), folder_lnk.id().to_string());
                self.state.read().modal_manager.open(Box::new(modal));
                *open = false;
            }

            ui.separator();

            if ui.button("✏  Rename").clicked() {
                let modal = CreateFolderModal::with_edit(
                    self.state.clone(),
                    folder_lnk.clone(),
                    self.folder_name.clone(),
                );
                self.state.read().modal_manager.open(Box::new(modal));
                *open = false;
            }

            if ui.button("👥  Duplicate").clicked() {
                FolderActions::duplicate_folder(self.state.clone(), folder_lnk.clone());
                *open = false;
            }

            ui.separator();

            let delete_btn = egui::Button::new(
                egui::RichText::new("🗑  Delete").color(Color32::from_rgb(255, 100, 100)),
            );

            if ui.add(delete_btn).clicked() {
                FolderActions::delete_folder(self.state.clone(), folder_lnk.clone());
                *open = false;
            }
        });
    }
}
