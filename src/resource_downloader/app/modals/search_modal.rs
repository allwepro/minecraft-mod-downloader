use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ListLnk, Project, ProjectLnk, ResourceType,
};
use crate::{
    get_list_mut, get_project_icon_texture, get_project_link, get_project_metadata, search_projects,
};
use eframe::egui;
use egui::{Id, Ui};

pub struct SearchModal {
    state: SharedRDState,
    list: ListLnk,
    resource_type: ResourceType,
    game_version: GameVersion,
    game_loader: GameLoader,
    search_query: String,
    searched_query: Option<String>,
    search_filter_exact: bool,
    project_to_add: Option<ProjectLnk>,
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
            list,
            resource_type,
            game_version,
            game_loader,
            search_query: String::new(),
            searched_query: None,
            search_filter_exact: true,
            project_to_add: None,
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

        egui::ScrollArea::vertical().show(ui, |ui| {
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
                if let Ok(searched) =
                    search_projects!(self.state, searched_query, self.resource_type, vd, ld)
                {
                    if let Some(results) = searched {
                        for project in results {
                            if let Ok(Some(data)) = get_project_metadata!(
                                self.state,
                                project.clone(),
                                self.resource_type
                            ) {
                                ui.horizontal(|ui| {
                                    if !data.icon_url.is_empty() {
                                        if let Some(handle) =
                                            get_project_icon_texture!(self.state, &project)
                                        {
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
                                    } else {
                                        ui.add_space(32.0);
                                    }
                                    ui.add_space(4.0);

                                    let button_width = 50.0;
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
                                        ui.label(format!(
                                            "👤 {} | ⬇ {}",
                                            data.author, data.download_count
                                        ));
                                    });

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("Add").clicked() {
                                                self.project_to_add = Some(project.clone());
                                                *open = false;
                                            }
                                        },
                                    );
                                });
                            }
                            ui.separator();
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new().size(48.0));
                            ui.add_space(10.0);
                            ui.label("Searching...");
                            ui.add_space(10.0);
                        });
                    }
                } else {
                    ui.label("Search failed. Please try again.");
                }
            } else {
                ui.label("Enter a search query");
            }
        });
    }

    fn on_close(&mut self) {
        if let Some(project) = self.project_to_add.take()
            && let Ok(Some(data)) =
                get_project_metadata!(self.state, project.clone(), self.resource_type)
        {
            get_list_mut!(self.state, &self.list).add_project(Project::new_from_rt_project(
                project,
                self.resource_type,
                true,
                data,
            ));
        }
    }
}
