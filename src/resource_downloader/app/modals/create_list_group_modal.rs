use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::domain::ListGroupLnk;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct CreateListGroupModal {
    state: SharedRDState,
    lg_name: String,
    save_on_close: bool,
    edit_mode: Option<ListGroupLnk>,
    parent_lg_lnk: Option<ListGroupLnk>,
}

impl CreateListGroupModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state,
            lg_name: String::new(),
            save_on_close: false,
            edit_mode: None,
            parent_lg_lnk: None,
        }
    }

    pub fn with_parent(state: SharedRDState, parent_lg_lnk: ListGroupLnk) -> Self {
        Self {
            state,
            lg_name: String::new(),
            save_on_close: false,
            edit_mode: None,
            parent_lg_lnk: Some(parent_lg_lnk),
        }
    }

    pub fn with_edit(state: SharedRDState, lg_lnk: ListGroupLnk, current_name: String) -> Self {
        Self {
            state,
            lg_name: current_name,
            save_on_close: false,
            edit_mode: Some(lg_lnk),
            parent_lg_lnk: None,
        }
    }
}

impl ModalWindow for CreateListGroupModal {
    fn id(&self) -> Id {
        if let Some(group) = &self.edit_mode {
            Id::new("rename_group").with(group.to_context_id())
        } else {
            Id::new("create_group")
        }
    }

    fn title(&self) -> String {
        if self.edit_mode.is_some() {
            "Rename Group".to_string()
        } else if self.parent_lg_lnk.is_some() {
            "Create Subgroup".to_string()
        } else {
            "Create Group".to_string()
        }
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.label("Group Name:");
        let response = ui.text_edit_singleline(&mut self.lg_name);

        if response.changed() && self.lg_name.starts_with(' ') {
            self.lg_name = self.lg_name.trim_start().to_string();
        }

        ui.add_space(12.0);

        let can_save = !self.lg_name.trim().is_empty();

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
            self.lg_name = "New Group".to_string();
        }
        self.save_on_close = false;
    }

    fn on_close(&mut self) {
        if !self.save_on_close {
            return;
        }

        let name = self.lg_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        if let Some(lg_lnk) = &self.edit_mode {
            ListGroupActions::rename_list_group(self.state.clone(), lg_lnk.clone(), name);
        } else {
            ListGroupActions::create_list_group_with_parent(
                self.state.clone(),
                name,
                self.parent_lg_lnk.clone(),
            );
        }
    }
}
