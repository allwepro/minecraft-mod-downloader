use crate::common::prefabs::modal_window::ModalWindow;
use crate::get_list;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{ListLnk, ProjectTypeConfig};
use egui::{Id, Ui};

#[derive(Clone)]
pub struct ListSettingsModal {
    state: SharedRDState,
    list_settings_component: ListSettingsComponent,
    list: ListLnk,
    save_on_close: bool,
}

impl ListSettingsModal {
    pub fn new(state: SharedRDState, list: ListLnk) -> Self {
        Self {
            state: state.clone(),
            list_settings_component: ListSettingsComponent::new_from_list(
                state.clone(),
                list.clone(),
            ),
            list,
            save_on_close: false,
        }
    }
}

impl ModalWindow for ListSettingsModal {
    fn id(&self) -> Id {
        Id::new("list_settings")
    }

    fn title(&self) -> String {
        "List Settings".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        self.list_settings_component.render_contents(ui);

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.save_on_close = true;
                *open = false;
            }
        });
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }
        let list_arc = get_list!(self.state, &self.list);
        let mut target_list = list_arc.write();
        target_list.set_game_version(
            self.list_settings_component
                .new_game_version
                .as_ref()
                .unwrap()
                .clone(),
        );
        target_list.set_resource_type(
            self.list_settings_component.new_resource_type,
            ProjectTypeConfig::new(
                self.list_settings_component
                    .new_game_loader
                    .as_ref()
                    .unwrap()
                    .clone(),
                self.list_settings_component.new_download_dir.clone(),
            ),
        );
        self.state.read().list_pool.save(&target_list.get_lnk());
    }
}
