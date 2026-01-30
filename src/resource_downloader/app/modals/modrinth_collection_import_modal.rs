use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{GameLoader, GameVersion, ResourceType};
use eframe::egui;
use egui::{Id, Ui};
use std::collections::HashMap;

#[derive(Clone)]
pub struct ModrinthCollectionImportModal {
    state: SharedRDState,
    list_settings_component: Option<ListSettingsComponent>,
    link: String,
    loading: bool,
    finalizing: bool,
    elements: HashMap<ResourceType, (GameVersion, GameLoader, Vec<String>)>,
}

impl ModrinthCollectionImportModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_settings_component: None,
            link: String::new(),
            loading: false,
            finalizing: false,
            elements: HashMap::new(),
        }
    }
    pub fn new_finalizing(
        state: SharedRDState,
        collection_id: String,
        elements: HashMap<ResourceType, (GameVersion, GameLoader, Vec<String>)>,
    ) -> Self {
        // Add the GameVersion, GameLoader suggestions for each ResourceType
        Self {
            state: state.clone(),
            list_settings_component: Some(ListSettingsComponent::new_with_rt(
                state.clone(),
                elements.keys().cloned().collect(),
            )),
            link: collection_id,
            loading: false,
            finalizing: true,
            elements,
        }
    }
}

impl ModalWindow for ModrinthCollectionImportModal {
    fn id(&self) -> Id {
        Id::new("import_modrinth_collection_modal")
    }

    fn title(&self) -> String {
        "Import Modrinth Collection".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(400.0);

        ui.label("Enter a Modrinth Collection URL or ID:");
        ui.add_space(4.0);

        let input_enabled = !self.loading || !self.finalizing;
        ui.add_enabled(
            input_enabled,
            egui::TextEdit::singleline(&mut self.link)
                .hint_text("https://modrinth.com/collection/ZCxg7r1U")
                .desired_width(ui.available_width()),
        );

        if self.finalizing {
            if self.elements.is_empty() {
                ui.add_space(8.0);
                ui.label("Collection contains no supported project types.");
            } else {
                if self.elements.len() > 1 {
                    ui.add_space(8.0);
                    ui.label("⚠ Collection contains multiple content types. Lists can contain only one content type. To import other types, please import them separately.");
                }
                ui.add_space(12.0);

                if let Some(component) = &mut self.list_settings_component {
                    component.render_contents(ui);
                }
            }
        }

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if self.loading {
                ui.add(egui::Spinner::new());
                ui.label("Loading collection...");
            } else {
                let can_import = parse_collection_id(&self.link).is_some();

                if ui
                    .add_enabled(can_import, egui::Button::new("Import"))
                    .clicked()
                {
                    self.loading = true;
                    if self.finalizing {
                        *open = false;
                    } else if self.loading {
                        self.state
                            .write()
                            .import_modrinth(parse_collection_id(&self.link).unwrap().to_string());
                    }
                }
            }
        });
    }

    fn on_close(&mut self) {
        if !self.finalizing {
            return;
        }

        self.state.read().list_pool.create_list(
            self.list_settings_component
                .as_ref()
                .unwrap()
                .new_list_name
                .clone(),
            self.list_settings_component
                .as_ref()
                .unwrap()
                .new_resource_type,
            self.list_settings_component
                .as_ref()
                .unwrap()
                .new_game_version
                .clone()
                .unwrap(),
            self.list_settings_component
                .as_ref()
                .unwrap()
                .new_game_loader
                .clone()
                .unwrap(),
            self.list_settings_component
                .as_ref()
                .unwrap()
                .new_download_dir
                .clone(),
            self.elements
                .get(
                    &self
                        .list_settings_component
                        .as_ref()
                        .unwrap()
                        .new_resource_type,
                )
                .unwrap()
                .2
                .clone(),
        );
    }
}

fn parse_collection_id(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    if input.contains("modrinth.com/collection/") {
        input
            .split("/collection/")
            .last()
            .map(|s| s.split(['/', '?', '#']).next().unwrap_or(s).to_string())
            .filter(|s| !s.is_empty())
    } else {
        Some(input.to_string())
    }
}
