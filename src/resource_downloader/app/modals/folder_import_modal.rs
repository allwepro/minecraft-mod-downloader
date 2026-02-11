use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::rd_state::FolderImportSession;
use crate::resource_downloader::domain::{RESOURCE_TYPES, ResourceType};
use egui::{Color32, Id, ScrollArea, Ui};
use std::collections::{HashMap, HashSet};

enum ImportStep {
    SelectResourceType,
    Scanning,
    Review,
    Settings,
}

pub struct FolderImportModal {
    state: SharedRDState,
    list_settings: Option<ListSettingsComponent>,
    step: ImportStep,
    selected_resource_type: ResourceType,
    selected_matches: HashMap<usize, usize>,
    skipped_items: HashSet<usize>,
    exact_matches: HashSet<usize>,
    manually_cleared: HashSet<usize>,
    show_only_unresolved: bool,
}

impl FolderImportModal {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state,
            list_settings: None,
            step: ImportStep::SelectResourceType,
            selected_resource_type: ResourceType::Mod,
            selected_matches: HashMap::new(),
            skipped_items: HashSet::new(),
            exact_matches: HashSet::new(),
            manually_cleared: HashSet::new(),
            show_only_unresolved: false,
        }
    }

    fn normalize_name(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect()
    }
}

impl ModalWindow for FolderImportModal {
    fn id(&self) -> Id {
        Id::new("folder_import_modal")
    }

    fn title(&self) -> String {
        "Import from Folder".to_string()
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        match &self.step {
            ImportStep::SelectResourceType => {
                ui.heading("Select Resource Type");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Resource Type:");
                    egui::ComboBox::from_id_salt("resource_type_selector")
                        .selected_text(format!(
                            "{} {}",
                            self.selected_resource_type.emoji(),
                            self.selected_resource_type.display_name()
                        ))
                        .show_ui(ui, |ui| {
                            for rt in RESOURCE_TYPES {
                                if ui
                                    .selectable_label(
                                        self.selected_resource_type == rt,
                                        format!("{} {}", rt.emoji(), rt.display_name()),
                                    )
                                    .clicked()
                                {
                                    self.selected_resource_type = rt;
                                }
                            }
                        });
                });

                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        let path_opt = self
                            .state
                            .read()
                            .folder_import_session
                            .as_ref()
                            .map(|s| s.path.clone());

                        if let Some(path) = path_opt {
                            self.state
                                .write()
                                .start_folder_import(path, Some(self.selected_resource_type));
                            self.step = ImportStep::Scanning;
                        }
                    }
                });
            }

            ImportStep::Scanning => {
                let session = self.state.read().folder_import_session.clone();

                if let Some(sess) = session {
                    if sess.is_scanning {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.spinner();
                            ui.heading("Scanning folder and searching for matches...");

                            if let Some((current, total, message)) = &sess.scan_progress {
                                ui.label(format!("Progress: {}/{}", current, total));
                                ui.label(message);
                            } else {
                                ui.label("Initializing scan...");
                            }

                            if let Some(error) = &sess.scan_error {
                                ui.add_space(10.0);
                                ui.label(egui::RichText::new(error).color(Color32::RED));
                            }

                            ui.add_space(20.0);
                        });
                    } else {
                        self.step = ImportStep::Review;
                    }
                }
            }

            ImportStep::Review => {
                let session = self.state.read().folder_import_session.clone();

                if let Some(sess) = session {
                    self.auto_select_exact_matches(&sess);

                    let unresolved_count = sess
                        .candidates
                        .iter()
                        .enumerate()
                        .filter(|(idx, _)| {
                            let is_skipped = self.skipped_items.contains(idx);
                            let has_selection = self.selected_matches.contains_key(idx);
                            !is_skipped && !has_selection
                        })
                        .count();

                    if unresolved_count == 0 {
                        self.show_only_unresolved = false;
                    }

                    ui.horizontal(|ui| {
                        ui.label(format!("Review {} detected files:", sess.candidates.len()));

                        if unresolved_count > 0 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let filter_text = if self.show_only_unresolved {
                                        "Show All".to_string()
                                    } else {
                                        format!("Show Unresolved ({})", unresolved_count)
                                    };

                                    if ui.button(filter_text).clicked() {
                                        self.show_only_unresolved = !self.show_only_unresolved;
                                    }
                                },
                            );
                        }
                    });

                    ui.separator();

                    let mut all_resolved = true;
                    let mut any_searching = false;

                    ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for (idx, candidate) in sess.candidates.iter().enumerate() {
                            let is_skipped = self.skipped_items.contains(&idx);
                            let has_selection = self.selected_matches.contains_key(&idx);
                            let is_resolved = is_skipped || has_selection;

                            if self.show_only_unresolved && is_resolved {
                                continue;
                            }

                            ui.group(|ui| {
                                ui.set_min_width(ui.available_width());
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&candidate.cleaned_name).strong(),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.small(format!("File: {}", candidate.original_filename));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.small(format!(
                                            "Detected: {} | {}",
                                            candidate.detected_version.name,
                                            candidate.detected_loader.name
                                        ));
                                    });

                                    if is_skipped {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("⊗ Skipped")
                                                    .color(Color32::GRAY),
                                            );
                                            if ui.small_button("Undo").clicked() {
                                                self.skipped_items.remove(&idx);
                                            }
                                        });
                                    } else {
                                        match &candidate.search_results {
                                            Some(matches) if !matches.is_empty() => {
                                                let selected_idx =
                                                    self.selected_matches.get(&idx).copied();

                                                if let Some(sel_idx) = selected_idx {
                                                    ui.horizontal(|ui| {
                                                        if self.exact_matches.contains(&idx) {
                                                            ui.label(
                                                                egui::RichText::new("Exact Match")
                                                                    .color(Color32::GREEN),
                                                            );
                                                        } else {
                                                            ui.label(
                                                                egui::RichText::new("Selected")
                                                                    .color(Color32::from_rgb(
                                                                        100, 200, 100,
                                                                    )),
                                                            );
                                                        }

                                                        if let Some((_, name)) =
                                                            matches.get(sel_idx)
                                                        {
                                                            ui.label(format!("- {}", name));
                                                        }

                                                        if matches.len() > 1
                                                            || !self.exact_matches.contains(&idx)
                                                                && ui
                                                                    .small_button("Change")
                                                                    .clicked()
                                                        {
                                                            self.selected_matches.remove(&idx);
                                                            self.exact_matches.remove(&idx);
                                                            self.manually_cleared.insert(idx);
                                                        }
                                                    });
                                                } else {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Select match:");
                                                        egui::ComboBox::from_id_salt(format!(
                                                            "match_{}",
                                                            idx
                                                        ))
                                                        .selected_text("Select...")
                                                        .show_ui(ui, |ui| {
                                                            for (m_idx, (_, name)) in
                                                                matches.iter().enumerate()
                                                            {
                                                                if ui
                                                                    .selectable_label(false, name)
                                                                    .clicked()
                                                                {
                                                                    self.selected_matches
                                                                        .insert(idx, m_idx);
                                                                    self.manually_cleared
                                                                        .remove(&idx);
                                                                }
                                                            }
                                                            if ui
                                                                .selectable_label(
                                                                    false,
                                                                    "⊗ Skip this file",
                                                                )
                                                                .clicked()
                                                            {
                                                                self.skipped_items.insert(idx);
                                                                self.manually_cleared.remove(&idx);
                                                            }
                                                        });
                                                    });

                                                    all_resolved = false;
                                                }
                                            }
                                            Some(_) => {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("No matches found")
                                                            .color(Color32::RED),
                                                    );
                                                    if ui.small_button("Skip").clicked() {
                                                        self.skipped_items.insert(idx);
                                                    }
                                                });
                                                if !is_skipped {
                                                    all_resolved = false;
                                                }
                                            }
                                            None => {
                                                any_searching = true;
                                                all_resolved = false;
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.label("Searching...");
                                                });
                                            }
                                        }
                                    }
                                });
                            });
                        }
                    });

                    ui.add_space(10.0);

                    if any_searching {
                        ui.label("Searching for matches...");
                    } else {
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(all_resolved, egui::Button::new("Continue"))
                                .clicked()
                            {
                                let default_name = sess
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();

                                let mut settings =
                                    ListSettingsComponent::new_wo_name_rt_with_default(
                                        self.state.clone(),
                                        self.selected_resource_type,
                                        default_name,
                                    );

                                if let Some(v) = sess.suggested_version {
                                    settings.new_game_version = Some(v);
                                }
                                if let Some(l) = sess.suggested_loader {
                                    settings.new_game_loader = Some(l);
                                }

                                self.list_settings = Some(settings);
                                self.step = ImportStep::Settings;
                            }
                        });
                    }
                }
            }

            ImportStep::Settings => {
                if let Some(settings) = &mut self.list_settings {
                    settings.render_contents(ui);

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Import").clicked() {
                            let name = settings.new_list_name.clone();
                            let version = settings.new_game_version.clone().unwrap();
                            let loader = settings.new_game_loader.clone().unwrap();
                            let dir = settings.new_download_dir.clone();

                            let projects: Vec<_> = {
                                let session = self.state.read().folder_import_session.clone();
                                if let Some(sess) = session {
                                    self.selected_matches
                                        .iter()
                                        .filter_map(|(idx, match_idx)| {
                                            sess.candidates
                                                .get(*idx)
                                                .and_then(|c| c.search_results.as_ref())
                                                .and_then(|matches| matches.get(*match_idx))
                                                .map(|(proj_lnk, _name)| proj_lnk.clone())
                                        })
                                        .collect()
                                } else {
                                    Vec::new()
                                }
                            };

                            {
                                let state = self.state.read();
                                state.create_import_folder_list(
                                    name,
                                    self.selected_resource_type,
                                    version,
                                    loader,
                                    dir,
                                    projects,
                                );
                            }

                            self.state.write().cancel_folder_import();
                            *open = false;
                        }
                    });
                }
            }
        }
    }

    fn on_close(&mut self) {
        self.state.write().cancel_folder_import();
    }
}

impl FolderImportModal {
    fn auto_select_exact_matches(&mut self, session: &FolderImportSession) {
        for (idx, candidate) in session.candidates.iter().enumerate() {
            if self.selected_matches.contains_key(&idx) || self.manually_cleared.contains(&idx) {
                continue;
            }

            if let Some(matches) = &candidate.search_results {
                let normalized_file_name = Self::normalize_name(&candidate.cleaned_name);

                if matches.len() == 1 {
                    if let Some((_, project_name)) = matches.first() {
                        let normalized_project_name = Self::normalize_name(project_name);
                        let length_diff = (normalized_file_name.len() as i32
                            - normalized_project_name.len() as i32)
                            .abs();

                        self.selected_matches.insert(idx, 0);

                        if normalized_file_name == normalized_project_name && length_diff <= 2 {
                            self.exact_matches.insert(idx);
                        }
                    }
                } else if matches.len() > 1 {
                    for (match_idx, (_, project_name)) in matches.iter().enumerate() {
                        let normalized_project_name = Self::normalize_name(project_name);
                        let length_diff = (normalized_file_name.len() as i32
                            - normalized_project_name.len() as i32)
                            .abs();

                        if normalized_file_name == normalized_project_name && length_diff <= 2 {
                            self.selected_matches.insert(idx, match_idx);
                            self.exact_matches.insert(idx);
                            break;
                        }
                    }
                }
            }
        }
    }
}
