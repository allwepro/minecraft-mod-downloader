use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct SettingsModal {
    state: SharedRDState,
    save_on_close: bool,
    default_list_name: String,
}

impl SettingsModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state,
            save_on_close: false,
            default_list_name: String::new(),
        }
    }
}

impl ModalWindow for SettingsModal {
    fn id(&self) -> Id {
        Id::new("settings")
    }

    fn title(&self) -> String {
        "Resource Downloader Settings".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.label("Default list name:");
        ui.text_edit_singleline(&mut self.default_list_name);

        ui.add_space(10.0);

        if ui.button("💾 Save Settings").clicked() {
            self.save_on_close = true;
            *open = false;
        }
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
        self.default_list_name = self.state.read().config.read().default_list_name.clone();
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }
        self.state.read().config.write().default_list_name = self.default_list_name.clone();
        self.state.write().save_config();
    }
}
