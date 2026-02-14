use crate::common::ui::structs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::ResourceType::Mod;
use egui::{Id, Ui};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct LegacyImportModal {
    state: SharedRDState,
    list_settings_component: ListSettingsComponent,
    path: PathBuf,
    save_on_close: bool,
}

impl LegacyImportModal {
    pub fn new(state: SharedRDState, path: PathBuf) -> Self {
        let default_name = Self::extract_name(&path).unwrap_or_default();

        Self {
            state: state.clone(),
            list_settings_component: ListSettingsComponent::new_wo_name_rt_with_default(
                state.clone(),
                Mod,
                default_name,
            ),
            path,
            save_on_close: false,
        }
    }

    fn extract_name(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
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
            self.list_settings_component.new_list_name.clone(),
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
    fn test_extract_name() {
        let cases = vec![
            ("/path/to/my_list.json", Some("my_list")),
            ("list.mmd", Some("list")),
            ("no_extension", Some("no_extension")),
            (".hidden", Some(".hidden")),
            ("", None),
        ];

        for (input, expected) in cases {
            let path = PathBuf::from(input);
            assert_eq!(
                LegacyImportModal::extract_name(&path),
                expected.map(|s| s.to_string()),
                "Failed for input: {}",
                input
            );
        }
    }
}
