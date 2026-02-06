use crate::resource_downloader::app::dialogs::Dialogs;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, RESOURCE_TYPES, ResourceType,
};
use crate::{get_default_dir, get_list, get_list_type, get_loaders, get_versions};
use egui::Ui;

#[derive(Clone)]
pub struct ListSettingsComponent {
    state: SharedRDState,
    resource_types: Vec<ResourceType>,
    hide_rt_and_name: bool,
    pub new_list_name: String,
    pub new_resource_type: ResourceType,
    pub new_game_version: Option<GameVersion>,
    pub new_game_loader: Option<GameLoader>,
    pub new_download_dir: String,
    pub new_download_dir_edited: bool,
}

impl ListSettingsComponent {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            hide_rt_and_name: false,
            resource_types: RESOURCE_TYPES.to_vec(),
            new_list_name: state.read().config.read().default_list_name.clone(),
            new_resource_type: ResourceType::Mod,
            new_game_version: None,
            new_game_loader: None,
            new_download_dir: String::new(),
            new_download_dir_edited: false,
        }
    }

    pub fn new_with_rt(state: SharedRDState, resource_type: Vec<ResourceType>) -> Self {
        Self {
            state: state.clone(),
            hide_rt_and_name: false,
            new_resource_type: resource_type
                .first()
                .cloned()
                .expect("No resource types provided"),
            resource_types: resource_type,
            new_list_name: state.read().config.read().default_list_name.clone(),
            new_game_version: None,
            new_game_loader: None,
            new_download_dir: String::new(),
            new_download_dir_edited: false,
        }
    }

    pub fn new_wo_name_rt(state: SharedRDState, resource_type: ResourceType) -> Self {
        Self {
            state,
            hide_rt_and_name: true,
            new_resource_type: resource_type,
            resource_types: vec![resource_type],
            new_list_name: String::new(),
            new_game_version: None,
            new_game_loader: None,
            new_download_dir: String::new(),
            new_download_dir_edited: false,
        }
    }

    pub fn new_from_list(state: SharedRDState, list: ListLnk) -> Self {
        let resource_type = { get_list_type!(state, &list) };
        let target_list = get_list!(state, &list);
        let loader = target_list
            .read()
            .get_resource_type_config(&resource_type)
            .expect("List without type")
            .loader
            .clone();
        let dir = target_list
            .read()
            .get_resource_type_config(&resource_type)
            .expect("List without type")
            .download_dir
            .clone();
        let ver = target_list.read().get_game_version().clone();
        Self {
            state,
            hide_rt_and_name: true,
            new_resource_type: resource_type,
            resource_types: vec![resource_type],
            new_list_name: String::new(),
            new_game_version: Some(ver),
            new_game_loader: Some(loader),
            new_download_dir: dir,
            new_download_dir_edited: true,
        }
    }

    pub fn reset(&mut self) {
        self.new_list_name = self.state.read().config.read().default_list_name.clone();
        self.new_game_version = None;
        self.new_game_loader = None;
        self.new_download_dir = String::new();
        self.new_download_dir_edited = false;
    }

    pub fn render_contents(&mut self, ui: &mut Ui) {
        if !self.hide_rt_and_name {
            ui.label("List Name:");
            ui.text_edit_singleline(&mut self.new_list_name);

            ui.add_space(10.0);

            ui.label("Content Type:");
            let rts = self.resource_types.clone();
            egui::ComboBox::from_id_salt("new_list_type_selector")
                .selected_text(self.new_resource_type.display_name())
                .show_ui(ui, |ui| {
                    for rt in rts {
                        if ui
                            .selectable_value(&mut self.new_resource_type, rt, rt.display_name())
                            .changed()
                        {
                            self.new_game_loader.take();

                            if !self.new_download_dir_edited {
                                self.new_download_dir = "".to_string();
                            }
                        }
                    }
                });

            ui.add_space(10.0);
        }

        ui.label("Minecraft Version:");
        let version_opt = get_versions!(self.state);
        match version_opt {
            Some(versions) => {
                if self.new_game_version.is_none() {
                    self.new_game_version = Some(versions[0].clone());
                }

                egui::ComboBox::from_id_salt("new_list_version_selector")
                    .selected_text(self.new_game_version.clone().unwrap().name)
                    .show_ui(ui, |ui| {
                        for version in versions {
                            ui.selectable_value(
                                &mut self.new_game_version,
                                Some(version.clone()),
                                &version.name,
                            );
                        }
                    });
            }
            None => {
                egui::ComboBox::from_id_salt("new_list_version_selector")
                    .selected_text("Loading...")
                    .show_ui(ui, |_ui| {});
            }
        }

        ui.add_space(10.0);

        ui.label("Loader:");

        let loaders_opt = get_loaders!(self.state, self.new_resource_type);
        match loaders_opt {
            Some(loaders) => {
                if self.new_game_loader.is_none() {
                    self.new_game_loader = Some(loaders[0].clone());
                }

                egui::ComboBox::from_id_salt("new_list_loader_selector")
                    .selected_text(self.new_game_loader.clone().unwrap().name)
                    .show_ui(ui, |ui| {
                        for loader in &loaders {
                            ui.selectable_value(
                                &mut self.new_game_loader,
                                Some(loader.clone()),
                                &loader.name,
                            );
                        }
                    });
            }
            None => {
                egui::ComboBox::from_id_salt("new_list_loader_selector")
                    .selected_text("Loading...")
                    .show_ui(ui, |_ui| {});
            }
        }

        ui.add_space(10.0);

        ui.label("Download Directory:");

        ui.horizontal(|ui| {
            if self.new_download_dir.is_empty() {
                self.new_download_dir =
                    get_default_dir!(self.state, &self.new_resource_type).clone();
            }

            if ui
                .text_edit_singleline(&mut self.new_download_dir)
                .changed()
            {
                self.new_download_dir_edited = true;
            }

            if ui.button("Browse...").clicked() {
                self.new_download_dir_edited =
                    Dialogs::pick_minecraft_mods_folder(&mut self.new_download_dir).is_some()
                        || self.new_download_dir_edited;
            }
        });
    }
}
