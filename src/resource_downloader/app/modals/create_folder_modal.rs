use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::folder_actions::FolderActions;
use crate::resource_downloader::domain::FolderLnk;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct CreateFolderModal {
    state: SharedRDState,
    folder_name: String,
    save_on_close: bool,
    edit_mode: Option<FolderLnk>,
    parent_id: Option<String>,
}

impl CreateFolderModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state,
            folder_name: String::new(),
            save_on_close: false,
            edit_mode: None,
            parent_id: None,
        }
    }

    pub fn with_parent(state: SharedRDState, parent_id: String) -> Self {
        Self {
            state,
            folder_name: String::new(),
            save_on_close: false,
            edit_mode: None,
            parent_id: Some(parent_id),
        }
    }

    pub fn with_edit(state: SharedRDState, folder_lnk: FolderLnk, current_name: String) -> Self {
        Self {
            state,
            folder_name: current_name,
            save_on_close: false,
            edit_mode: Some(folder_lnk),
            parent_id: None,
        }
    }
}

impl ModalWindow for CreateFolderModal {
    fn id(&self) -> Id {
        if let Some(folder) = &self.edit_mode {
            Id::new("rename_folder").with(folder.id())
        } else {
            Id::new("create_folder")
        }
    }

    fn title(&self) -> String {
        if self.edit_mode.is_some() {
            "Rename Folder".to_string()
        } else if self.parent_id.is_some() {
            "Create Subfolder".to_string()
        } else {
            "Create Folder".to_string()
        }
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.label("Folder Name:");
        let response = ui.text_edit_singleline(&mut self.folder_name);

        if response.changed() && self.folder_name.starts_with(' ') {
            self.folder_name = self.folder_name.trim_start().to_string();
        }

        ui.add_space(12.0);

        let can_save = !self.folder_name.trim().is_empty();

        let button_text = if self.edit_mode.is_some() {
            "Rename"
        } else {
            "Create"
        };

        if ui
            .add_enabled(can_save, egui::Button::new(button_text))
            .clicked()
        {
            self.save_on_close = true;
            *open = false;
        }
    }

    fn on_open(&mut self) {
        if self.edit_mode.is_none() {
            self.folder_name = "New Folder".to_string();
        }
        self.save_on_close = false;
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }

        let name = self.folder_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        if let Some(folder_lnk) = &self.edit_mode {
            FolderActions::rename_folder(self.state.clone(), folder_lnk.clone(), name);
        } else {
            FolderActions::create_folder_with_parent(
                self.state.clone(),
                name,
                self.parent_id.clone(),
            );
        }
    }
}
