use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct CreateModal {
    state: SharedRDState,
    list_settings_component: ListSettingsComponent,
    save_on_close: bool,
}

impl CreateModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_settings_component: ListSettingsComponent::new(state.clone()),
            save_on_close: false,
        }
    }
}

impl ModalWindow for CreateModal {
    fn id(&self) -> Id {
        Id::new("create_list")
    }

    fn title(&self) -> String {
        "Create List".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        self.list_settings_component.render_contents(ui);

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Create").clicked() {
                self.save_on_close = true;
                *open = false;
            }
        });
    }

    fn on_open(&mut self) {
        self.list_settings_component.reset();
        self.save_on_close = false;
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }
        self.state.read().list_pool.create_list(
            self.list_settings_component.new_list_name.clone(),
            self.list_settings_component.new_resource_type,
            self.list_settings_component
                .new_game_version
                .clone()
                .unwrap(),
            self.list_settings_component
                .new_game_loader
                .clone()
                .unwrap(),
            self.list_settings_component.new_download_dir.clone(),
            vec![],
        );
    }
}
