use crate::common::ui::structs::popup_window::Popup;
use crate::resource_downloader::app::modals::create_list_group_modal::CreateListGroupModal;
use crate::resource_downloader::app::modals::create_modal::CreateModal;
use crate::resource_downloader::business::SharedRDState;
use eframe::egui;
use eframe::epaint::Color32;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct CreateMenuPopup {
    state: SharedRDState,
    new_list_modal: CreateModal,
    new_list_group_modal: CreateListGroupModal,
}

impl CreateMenuPopup {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            new_list_modal: CreateModal::new(state.clone()),
            new_list_group_modal: CreateListGroupModal::new(state.clone()),
        }
    }
}

impl Popup for CreateMenuPopup {
    fn id(&self) -> Id {
        Id::new("create_menu_popup")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(140.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            if ui
                .add(
                    egui::Button::new("📋  New List")
                        .min_size(egui::vec2(150.0, 0.0))
                        .fill(Color32::TRANSPARENT),
                )
                .clicked()
            {
                self.state
                    .read()
                    .modal_manager
                    .open(Box::new(self.new_list_modal.clone()));
                *open = false;
            }

            if ui
                .add(
                    egui::Button::new("📁  New Group")
                        .min_size(egui::vec2(150.0, 0.0))
                        .fill(Color32::TRANSPARENT),
                )
                .clicked()
            {
                self.state
                    .read()
                    .modal_manager
                    .open(Box::new(self.new_list_group_modal.clone()));
                *open = false;
            }
        });
    }
}
