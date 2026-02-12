use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
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
            let can_create = self.list_settings_component.new_game_version.is_some()
                && self.list_settings_component.new_game_loader.is_some();

            if ui
                .add_enabled(can_create, egui::Button::new("Create"))
                .clicked()
            {
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

        let (ver, loader) = match (
            self.list_settings_component.new_game_version.clone(),
            self.list_settings_component.new_game_loader.clone(),
        ) {
            (Some(v), Some(l)) => (v, l),
            _ => return,
        };

        ListActions::create_list(
            self.state.clone(),
            self.list_settings_component.new_list_name.clone(),
            self.list_settings_component.new_resource_type,
            ver,
            loader,
            self.list_settings_component.new_download_dir.clone(),
        );
    }
}
