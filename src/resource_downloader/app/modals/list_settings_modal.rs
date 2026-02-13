use crate::common::ui::structs::modal_window::ModalWindow;
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
            let can_save = self.list_settings_component.new_game_version.is_some()
                && self.list_settings_component.new_game_loader.is_some();

            if ui
                .add_enabled(can_save, egui::Button::new("Save"))
                .clicked()
            {
                self.save_on_close = true;
                *open = false;
            }
        });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("🆔 List ID: {}", self.list))
                .small()
                .color(egui::Color32::GRAY),
        );
    }

    fn on_open(&mut self) {
        self.save_on_close = false;
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }

        let pool = self.state.read().list_pool.clone();
        let lnk = self.list.clone();

        let (ver, loader) = match (
            self.list_settings_component.new_game_version.clone(),
            self.list_settings_component.new_game_loader.clone(),
        ) {
            (Some(v), Some(l)) => (v, l),
            _ => return,
        };

        if let Some(list_arc) = pool.get(&lnk) {
            let mut target_list = list_arc.write();
            target_list.set_game_version(ver);
            target_list.set_resource_type(
                self.list_settings_component.new_resource_type,
                ProjectTypeConfig::new(
                    loader,
                    self.list_settings_component.new_download_dir.clone(),
                ),
            );
            target_list.set_do_updates(Some(self.list_settings_component.new_do_updates));
            drop(target_list);
            pool.save(&lnk);
        }

        self.state.write().request_full_refresh();
    }
}
