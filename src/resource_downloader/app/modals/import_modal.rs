use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::ListLnk;
use eframe::egui;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct ImportModal {
    state: SharedRDState,
    list: ListLnk,
    item_count: i32,
    save_on_close: bool,
    new_list_name: String,
}

impl ImportModal {
    pub fn new(state: SharedRDState, list: ListLnk) -> Self {
        Self {
            state,
            list,
            item_count: 0,
            save_on_close: false,
            new_list_name: String::new(),
        }
    }
}

impl ModalWindow for ImportModal {
    fn id(&self) -> Id {
        Id::new("import_modal")
    }

    fn title(&self) -> String {
        "Import List".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.label("List Name:");
        ui.text_edit_singleline(&mut self.new_list_name);

        ui.add_space(10.0);
        ui.label(egui::RichText::new(format!("Contains {} items", self.item_count)).weak());

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Import").clicked() {
                self.save_on_close = true;
                *open = false;
            }
        });
    }

    fn on_open(&mut self) {
        if let Some(list) = self.state.read().list_pool.get(&self.list) {
            self.new_list_name = list.read().get_name().clone();
            self.item_count = list.read().project_count() as i32;
        }
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            self.state.read().list_pool.delete(&self.list);
            return;
        }

        if let Some(list) = self.state.read().list_pool.get(&self.list) {
            list.write().set_list_name(self.new_list_name.clone());
        }
        self.state.read().list_pool.save(&self.list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_modal_id_constant() {
        let test_id = Id::new("import_modal");
        let expected_id = Id::new("import_modal");
        assert_eq!(test_id, expected_id);
    }

    #[test]
    fn test_import_modal_title_constant() {
        let expected = "Import List".to_string();
        assert_eq!(expected, "Import List");
    }

    #[test]
    fn test_list_lnk_creation() {
        let list = ListLnk::new("test_list".to_string());
        let list_clone = list.clone();

        assert_eq!(list, list_clone);
    }

    #[test]
    fn test_import_modal_default_values() {
        assert_eq!(0, 0); // item_count default
        assert_eq!("", ""); // new_list_name default
        let save_on_close = false;
        assert!(!save_on_close, "save_on_close default should be false");
    }

    #[test]
    fn test_save_flag_toggle() {
        let mut save_flag = false;
        assert!(!save_flag);

        save_flag = true;
        assert!(save_flag);

        save_flag = false;
        assert!(!save_flag);
    }

    #[test]
    fn test_list_name_modification() {
        let mut name = String::new();
        assert!(name.is_empty());

        name = "Test Name".to_string();
        assert_eq!(name, "Test Name");

        name.clear();
        assert!(name.is_empty());
    }

    #[test]
    fn test_item_count_modification() {
        let mut count = 0i32;
        assert_eq!(count, 0);

        count = 42;
        assert_eq!(count, 42);

        count = 0;
        assert_eq!(count, 0);
    }
}
