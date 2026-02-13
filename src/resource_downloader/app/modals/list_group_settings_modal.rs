use crate::common::ui::structs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::ListGroupLnk;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct ListGroupSettingsModal {
    _state: SharedRDState,
    lg_lnk: ListGroupLnk,
    save_on_close: bool,
}

impl ListGroupSettingsModal {
    pub fn new(state: SharedRDState, lg_lnk: ListGroupLnk) -> Self {
        Self {
            _state: state,
            lg_lnk,
            save_on_close: false,
        }
    }
}

impl ModalWindow for ListGroupSettingsModal {
    fn id(&self) -> Id {
        Id::new("list_group_settings").with(self.lg_lnk.to_context_id())
    }

    fn title(&self) -> String {
        "List Group Settings".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, _open: &mut bool) {
        /*ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.save_on_close = true;
                *open = false;
            }
        });*/
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("🆔 List Group ID: {}", self.lg_lnk.to_context_id()))
                .small()
                .color(egui::Color32::GRAY),
        );
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
    }

    fn on_close(&mut self) {
        if !self.save_on_close {}
    }
}
