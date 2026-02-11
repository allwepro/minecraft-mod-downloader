use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::project_actions::ProjectActions;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, ProjectLnk, ResourceType,
};
use crate::{get_project_icon_texture, get_project_link, get_project_metadata, search_projects};
use eframe::egui;
use eframe::epaint::Color32;
use egui::{Id, Ui};

pub type SearchSelectionCallback = Box<dyn FnOnce(&SharedRDState, ProjectLnk) + Send + Sync>;
pub type SearchCloseCallback = Box<dyn FnOnce(&SharedRDState) + Send + Sync>;

pub struct SearchModal {
    state: SharedRDState,
    list: Option<ListLnk>,
    resource_type: ResourceType,
    game_version: GameVersion,
    game_loader: GameLoader,
    search_query: String,
    searched_query: Option<String>,
    search_filter_exact: bool,
    project_to_add: Option<ProjectLnk>,
    callback: Option<SearchSelectionCallback>,
    close_callback: Option<SearchCloseCallback>,
}

impl SearchModal {
    pub fn new(
        state: SharedRDState,
        list: ListLnk,
        resource_type: ResourceType,
        game_version: GameVersion,
        game_loader: GameLoader,
    ) -> Self {
        Self {
            state,
            list: Some(list),
            resource_type,
            game_version,
            game_loader,
            search_query: String::new(),
            searched_query: None,
            search_filter_exact: true,
            project_to_add: None,
            callback: None,
            close_callback: None,
        }
    }

    pub fn new_with_callback(
        state: SharedRDState,
        resource_type: ResourceType,
        game_version: GameVersion,
        game_loader: GameLoader,
        initial_query: String,
        callback: SearchSelectionCallback,
        close_callback: SearchCloseCallback,
    ) -> Self {
        let mut searched_query = None;
        if !initial_query.is_empty() {
            searched_query = Some(initial_query.clone());
        }

        Self {
            state,
            list: None,
            resource_type,
            game_version,
            game_loader,
            search_query: initial_query,
            searched_query,
            search_filter_exact: false,
            project_to_add: None,
            callback: Some(callback),
            close_callback: Some(close_callback),
        }
    }
}

impl ModalWindow for SearchModal {
    fn id(&self) -> Id {
        Id::new("search_projects")
    }

    fn title(&self) -> String {
        "Search Project".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.horizontal(|ui| {
            let query_response = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search name or description...")
                    .desired_width(400.0),
            );

            if query_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.searched_query = Some(self.search_query.clone());
            }

            ui.checkbox(&mut self.search_filter_exact, "Match version/loader");

            if ui.button("Search").clicked() {
                self.searched_query = Some(self.search_query.clone());
            }
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(searched_query) = &self.searched_query {
                    let ld = if self.search_filter_exact {
                        Some(self.game_loader.clone())
                    } else {
                        None
                    };
                    let vd = if self.search_filter_exact {
                        Some(self.game_version.clone())
                    } else {
                        None
                    };

                    let searched =
                        search_projects!(self.state, searched_query, self.resource_type, vd, ld);

                    if let Ok(Some(results)) = searched {
                        if results.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(
                                    egui::RichText::new("No projects found matching your search.")
                                        .weak(),
                                );
                            });
                        }

                        for project in results {
                            let metadata = get_project_metadata!(
                                self.state,
                                project.clone(),
                                self.resource_type
                            );

                            match metadata {
                                Ok(Some(data)) => {
                                    ui.horizontal(|ui| {
                                        let icon_tex = get_project_icon_texture!(
                                            self.state, &project, &data.name
                                        );

                                        if let Some(handle) = icon_tex {
                                            ui.add(
                                                egui::Image::from_texture(&handle)
                                                    .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                                            );
                                        } else {
                                            ui.add_sized(
                                                egui::vec2(32.0, 32.0),
                                                egui::Spinner::new(),
                                            );
                                        }
                                        ui.add_space(4.0);

                                        let button_width = 80.0;
                                        let spacing = 8.0;
                                        let available_width =
                                            ui.available_width() - button_width - spacing;

                                        ui.vertical(|ui| {
                                            ui.set_max_width(available_width);
                                            let project_link = get_project_link!(
                                                self.state,
                                                &project,
                                                &self.resource_type
                                            );
                                            ui.hyperlink_to(&data.name, project_link);
                                            ui.add(
                                                egui::Label::new(&data.description)
                                                    .wrap_mode(egui::TextWrapMode::Wrap),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "👤 {} | ⬇ {}",
                                                    data.author, data.download_count
                                                ))
                                                .small()
                                                .weak(),
                                            );
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let btn_text = if self.callback.is_some() {
                                                    "Select"
                                                } else {
                                                    "Add"
                                                };

                                                if ui
                                                    .button(
                                                        egui::RichText::new(btn_text)
                                                            .color(Color32::LIGHT_GREEN),
                                                    )
                                                    .clicked()
                                                {
                                                    self.project_to_add = Some(project.clone());
                                                    *open = false;
                                                }
                                            },
                                        );
                                    });
                                }
                                Ok(None) => {
                                    ui.horizontal(|ui| {
                                        ui.add_sized(egui::vec2(32.0, 32.0), egui::Spinner::new());
                                        ui.label("Loading project details...");
                                    });
                                }
                                Err(_) => {
                                    ui.label(
                                        egui::RichText::new("Failed to load project details")
                                            .color(Color32::RED),
                                    );
                                }
                            }
                            ui.separator();
                        }
                    } else if let Ok(None) = searched {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new().size(48.0));
                            ui.add_space(10.0);
                            ui.label("Searching...");
                            ui.add_space(10.0);
                        });
                    } else {
                        ui.label("Search failed. Please try again.");
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("Enter a search query above.").weak());
                    });
                }
            });
    }

    fn on_close(&mut self) {
        if let Some(project) = self.project_to_add.take() {
            if let Some(callback) = self.callback.take() {
                callback(&self.state, project);
                return;
            }

            if let Some(list) = &self.list {
                let metadata =
                    get_project_metadata!(self.state, project.clone(), self.resource_type);
                if let Ok(Some(data)) = metadata {
                    ProjectActions::add_project(
                        self.state.clone(),
                        list.clone(),
                        project.clone(),
                        self.resource_type,
                        data,
                    );
                }
            }
        } else if let Some(close_callback) = self.close_callback.take() {
            close_callback(&self.state);
        }
    }
}
