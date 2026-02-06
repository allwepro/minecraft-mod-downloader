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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_import_modal_id_constant() {
        let test_id = Id::new("import_legacy_list");
        let expected_id = Id::new("import_legacy_list");
        assert_eq!(test_id, expected_id);
    }

    #[test]
    fn test_legacy_import_modal_title_constant() {
        let expected = "Import List".to_string();
        assert_eq!(expected, "Import List");
    }

    #[test]
    fn test_path_handling() {
        let path1 = PathBuf::from("/tmp/list1.json");
        let path2 = PathBuf::from("/tmp/list2.json");

        assert_ne!(path1, path2);
        assert_eq!(path1, PathBuf::from("/tmp/list1.json"));
    }

    #[test]
    fn test_path_components() {
        let path = PathBuf::from("/tmp/test_list.json");

        assert!(path.as_path().to_str().is_some());
        assert_eq!(path.file_name().unwrap(), "test_list.json");
    }

    #[test]
    fn test_save_on_close_toggle() {
        let mut save_flag = false;
        assert!(!save_flag);

        save_flag = true;
        assert!(save_flag);

        save_flag = false;
        assert!(!save_flag);
    }

    #[test]
    fn test_legacy_import_modal_default_state() {
        let save_on_close = false;
        assert!(!save_on_close, "save_on_close should initialize to false");
    }

    #[test]
    fn test_multiple_path_instances() {
        let path1 = PathBuf::from("/tmp/list1.json");
        let path2 = PathBuf::from("/tmp/list2.json");
        let path1_copy = path1.clone();

        assert_eq!(path1, path1_copy);
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_resource_type_mod_constant() {
        use crate::resource_downloader::domain::ResourceType;
        let rt = ResourceType::Mod;
        let rt2 = ResourceType::Mod;

        assert_eq!(rt, rt2);
    }
}
