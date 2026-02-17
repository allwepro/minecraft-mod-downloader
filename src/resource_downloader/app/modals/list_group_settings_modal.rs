use crate::common::ui::structs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::domain::ListGroupLnk;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct ListGroupSettingsModal {
    state: SharedRDState,
    lg_lnk: ListGroupLnk,
    save_on_close: bool,
}

impl ListGroupSettingsModal {
    pub fn new(state: SharedRDState, lg_lnk: ListGroupLnk) -> Self {
        Self {
            state,
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
        let is_instance =
            ListGroupActions::is_instance_mode(self.state.clone(), self.lg_lnk.clone());

        if is_instance {
            "Instance Settings".to_string()
        } else {
            "List Group Settings".to_string()
        }
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        let is_instance =
            ListGroupActions::is_instance_mode(self.state.clone(), self.lg_lnk.clone());

        ui.add_space(8.0);

        if is_instance {
            if ui.button("↩ Revert to Group").clicked() {
                ListGroupActions::toggle_instance_mode(self.state.clone(), self.lg_lnk.clone());
                *open = false;
            }
        } else if ui.button("🎮 Convert to Instance").clicked() {
            ListGroupActions::toggle_instance_mode(self.state.clone(), self.lg_lnk.clone());
            *open = false;
        }

        ui.add_space(8.0);
        ui.separator();
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
