use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{ListLnk, ResourceType};
use eframe::egui;
use egui::{Color32, Id, Ui};

#[derive(Clone)]
pub struct ListContextMenu {
    state: SharedRDState,
    list_lnk: ListLnk,
}

impl ListContextMenu {
    pub fn new(state: SharedRDState, list_lnk: ListLnk) -> Self {
        Self { state, list_lnk }
    }
}

impl Popup for ListContextMenu {
    fn id(&self) -> Id {
        Id::new("list_context_menu").with(&self.list_lnk)
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        let list_lnk = self.list_lnk.clone();
        let list_pool = self.state.read().list_pool.clone();

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            if ui.button("📂  Open Folder").clicked() {
                if let Some(list_arc) = list_pool.get(&list_lnk) {
                    let list = list_arc.read();
                    let rt = list
                        .get_resource_types()
                        .first()
                        .cloned()
                        .unwrap_or(ResourceType::Mod);
                    if let Some(config) = list.get_resource_type_config(&rt) {
                        self.state
                            .read()
                            .open_explorer(config.download_dir.clone().into());
                    }
                }
                *open = false;
            }

            if ui.button("👥  Duplicate").clicked() {
                list_pool.duplicate(&list_lnk);
                *open = false;
            }

            ui.separator();

            let delete_btn = egui::Button::new(
                egui::RichText::new("🗑  Delete").color(Color32::from_rgb(255, 100, 100)),
            );

            if ui.add(delete_btn).clicked() {
                if self.state.read().open_list.as_ref() == Some(&list_lnk) {
                    self.state.write().set_open_list_no_save(None);
                }
                list_pool.delete(&list_lnk);
                *open = false;
            }
        });
    }
}
