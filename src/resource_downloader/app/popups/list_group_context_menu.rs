use crate::common::ui::structs::popup_window::Popup;
use crate::resource_downloader::app::modals::create_list_group_modal::CreateListGroupModal;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::domain::ListGroupLnk;
use eframe::egui;
use egui::{Color32, Id, Ui};

#[derive(Clone)]
pub struct ListGroupContextMenu {
    state: SharedRDState,
    lg_lnk: ListGroupLnk,
    lg_name: String,
}

impl ListGroupContextMenu {
    pub fn new(state: SharedRDState, lg_lnk: ListGroupLnk, lg_name: String) -> Self {
        Self {
            state,
            lg_lnk,
            lg_name,
        }
    }
}

impl Popup for ListGroupContextMenu {
    fn id(&self) -> Id {
        Id::new("list_group_context_menu").with(&self.lg_lnk)
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        let lg_lnk = self.lg_lnk.clone();

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            if ui.button("📁  Create Subgroup").clicked() {
                let modal = CreateListGroupModal::with_parent(self.state.clone(), lg_lnk.clone());
                self.state.read().modal_manager.open(Box::new(modal));
                *open = false;
            }

            ui.separator();

            if ui.button("✏  Rename").clicked() {
                let modal = CreateListGroupModal::with_edit(
                    self.state.clone(),
                    lg_lnk.clone(),
                    self.lg_name.clone(),
                );
                self.state.read().modal_manager.open(Box::new(modal));
                *open = false;
            }

            if ui.button("👥  Duplicate").clicked() {
                ListGroupActions::duplicate_list_group(self.state.clone(), lg_lnk.clone());
                *open = false;
            }

            ui.separator();

            let delete_btn = egui::Button::new(
                egui::RichText::new("🗑  Delete").color(Color32::from_rgb(255, 100, 100)),
            );

            if ui.add(delete_btn).clicked() {
                ListGroupActions::delete_list_group(self.state.clone(), lg_lnk.clone());
                *open = false;
            }
        });
    }
}
