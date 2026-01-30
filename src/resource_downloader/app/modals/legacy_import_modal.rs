use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::ResourceType::Mod;
use egui::{Id, Ui};
use std::path::PathBuf;

#[derive(Clone)]
pub struct LegacyImportModal {
    state: SharedRDState,
    list_settings_component: ListSettingsComponent,
    path: PathBuf,
    save_on_close: bool,
}

impl LegacyImportModal {
    pub fn new(state: SharedRDState, path: PathBuf) -> Self {
        Self {
            state: state.clone(),
            list_settings_component: ListSettingsComponent::new_wo_name_rt(state.clone(), Mod),
            path,
            save_on_close: false,
        }
    }
}

impl ModalWindow for LegacyImportModal {
    fn id(&self) -> Id {
        Id::new("import_legacy_list")
    }

    fn title(&self) -> String {
        "Import List".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        self.list_settings_component.render_contents(ui);

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Import").clicked() {
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
        self.state.read().list_pool.import_legacy(
            self.path.clone(),
            self.list_settings_component
                .new_game_version
                .clone()
                .unwrap(),
            self.list_settings_component
                .new_game_loader
                .clone()
                .unwrap(),
            self.list_settings_component.new_download_dir.clone(),
        );
    }
}
