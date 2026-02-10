use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::domain::ListLnk;
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
            if ui.button("📂  Open Folder").clicked() {
                ListActions::open_folder(self.state.clone(), list_lnk.clone());
                *open = false;
            }

            if ui.button("👥  Duplicate").clicked() {
                ListActions::duplicate_list(self.state.clone(), list_lnk.clone());
                *open = false;
            }

            ui.separator();

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
