use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::app::components::list_settings_component::ListSettingsComponent;
use crate::resource_downloader::app::dialogs::Dialogs;
use crate::resource_downloader::app::modals::search_modal::{
    SearchCloseCallback, SearchModal, SearchSelectionCallback,
};
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::rd_state::FolderImportSession;
use crate::resource_downloader::domain::{RESOURCE_TYPES, ResourceType};
use crate::{get_default_dir, get_project_metadata};
use egui::{Color32, Id, ScrollArea, Ui};
use std::path::PathBuf;

enum ImportStep {
    SelectFolder,
    Scanning,
    Review,
    Settings,
}

pub struct FolderImportModal {
    state: SharedRDState,
    list_settings: Option<ListSettingsComponent>,
    step: ImportStep,
    selected_resource_type: ResourceType,
    selected_folder: String,
    transitioning_to_search: bool,
}

impl FolderImportModal {
    pub fn new(state: SharedRDState) -> Self {
        let (selected_resource_type, selected_folder, step) = {
            let s = state.read();
            if let Some(sess) = &s.folder_import_session {
                let step = if sess.is_scanning {
                    ImportStep::Scanning
                } else {
                    ImportStep::Review
                };
                (sess.resource_type, sess.path.display().to_string(), step)
            } else {
                let rt = ResourceType::Mod;
                let folder = get_default_dir!(state, &rt);
                (rt, folder, ImportStep::SelectFolder)
            }
        };

        Self {
            state,
            list_settings: None,
            step,
            selected_resource_type,
            selected_folder,
            transitioning_to_search: false,
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
            ImportStep::SelectFolder => {
                ui.heading("Select Folder to Import");
                ui.add_space(10.0);

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
                                self.selected_folder =
                                    get_default_dir!(self.state, &self.selected_resource_type);
                            }
                        }
                    });

                ui.add_space(10.0);

                ui.label("Folder:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.selected_folder);

                    if ui.button("Browse...").clicked()
                        && let Some(path) = Dialogs::pick_folder(&mut self.selected_folder)
                    {
                        self.selected_folder = path.display().to_string();
                    }
                });

                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        let path = PathBuf::from(&self.selected_folder);
                        self.state
                            .write()
                            .start_folder_import(path, Some(self.selected_resource_type));
                        self.step = ImportStep::Scanning;
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
                let session_opt = self.state.read().folder_import_session.clone();

                if let Some(mut sess) = session_opt {
                    self.auto_select_exact_matches(&mut sess);

                    let unresolved_count = sess
                        .candidates
                        .iter()
                        .enumerate()
                        .filter(|(idx, _)| {
                            let is_skipped = sess.skipped_items.contains(idx);
                            let has_selection = sess.selected_matches.contains_key(idx);
                            !is_skipped && !has_selection
                        })
                        .count();

                    if unresolved_count == 0 {
                        sess.show_only_unresolved = false;
                    }

                    ui.horizontal(|ui| {
                        ui.label(format!("Review {} detected files:", sess.candidates.len()));

                        if unresolved_count > 0 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let filter_text = if sess.show_only_unresolved {
                                        "Show All".to_string()
                                    } else {
                                        format!("Show Unresolved ({})", unresolved_count)
                                    };

                                    if ui.button(filter_text).clicked() {
                                        sess.show_only_unresolved = !sess.show_only_unresolved;
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
                            let is_skipped = sess.skipped_items.contains(&idx);
                            let has_selection = sess.selected_matches.contains_key(&idx);
                            let is_resolved = is_skipped || has_selection;

                            if sess.show_only_unresolved && is_resolved {
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
                                                sess.skipped_items.remove(&idx);
                                            }
                                        });
                                    } else {
                                        match &candidate.search_results {
                                            Some(matches) if !matches.is_empty() => {
                                                let selected_idx =
                                                    sess.selected_matches.get(&idx).copied();

                                                if let Some(sel_idx) = selected_idx {
                                                    ui.horizontal(|ui| {
                                                        if sess.exact_matches.contains(&idx) {
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

                                                        if (matches.len() > 1
                                                            || !sess.exact_matches.contains(&idx))
                                                            && ui.small_button("Change").clicked()
                                                        {
                                                            sess.selected_matches.remove(&idx);
                                                            sess.exact_matches.remove(&idx);
                                                            sess.manually_cleared.insert(idx);
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
                                                                    sess.selected_matches
                                                                        .insert(idx, m_idx);
                                                                    sess.manually_cleared
                                                                        .remove(&idx);
                                                                }
                                                            }

                                                            if ui
                                                                .selectable_label(
                                                                    false,
                                                                    "🔍 Search Manually...",
                                                                )
                                                                .clicked()
                                                            {
                                                                self.open_search_for_candidate(idx);
                                                            }

                                                            if ui
                                                                .selectable_label(
                                                                    false,
                                                                    "⊗ Skip this file",
                                                                )
                                                                .clicked()
                                                            {
                                                                sess.skipped_items.insert(idx);
                                                                sess.manually_cleared.remove(&idx);
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

                                                    if ui.small_button("🔍 Search").clicked() {
                                                        self.open_search_for_candidate(idx);
                                                    }

                                                    if ui.small_button("Skip").clicked() {
                                                        sess.skipped_items.insert(idx);
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

                    self.state.write().folder_import_session = Some(sess.clone());

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

                                let mut settings = ListSettingsComponent::new_wo_rt_with_default(
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
                                    sess.selected_matches
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
        if !self.transitioning_to_search {
            self.state.write().cancel_folder_import();
        }
    }
}

impl FolderImportModal {
    fn open_search_for_candidate(&mut self, candidate_idx: usize) {
        let (query, rt, ver, loader) = {
            let s = self.state.read();
            let sess = s.folder_import_session.as_ref().unwrap();
            let cand = &sess.candidates[candidate_idx];
            (
                cand.cleaned_name.clone(),
                sess.resource_type,
                cand.detected_version.clone(),
                cand.detected_loader.clone(),
            )
        };

        self.transitioning_to_search = true;

        let callback: SearchSelectionCallback = Box::new(move |state, project_lnk| {
            let metadata = get_project_metadata!(state, project_lnk.clone(), rt);
            if let Ok(Some(data)) = metadata {
                let mut s = state.write();
                if let Some(sess) = &mut s.folder_import_session {
                    let cand = &mut sess.candidates[candidate_idx];
                    let results = cand.search_results.get_or_insert_with(Vec::new);
                    let match_idx = results.len();
                    results.push((project_lnk, data.name));
                    sess.selected_matches.insert(candidate_idx, match_idx);
                    sess.manually_cleared.remove(&candidate_idx);
                }
            }
            state
                .read()
                .submit_modal(Box::new(FolderImportModal::new(state.clone())));
        });

        let close_callback: SearchCloseCallback = Box::new(move |state| {
            state
                .read()
                .submit_modal(Box::new(FolderImportModal::new(state.clone())));
        });

        let search_modal = SearchModal::new_with_callback(
            self.state.clone(),
            rt,
            ver,
            loader,
            query,
            callback,
            close_callback,
        );

        self.state.read().submit_modal(Box::new(search_modal));
    }

    fn auto_select_exact_matches(&mut self, session: &mut FolderImportSession) {
        for (idx, candidate) in session.candidates.iter().enumerate() {
            if session.selected_matches.contains_key(&idx)
                || session.manually_cleared.contains(&idx)
            {
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

                        session.selected_matches.insert(idx, 0);

                        if normalized_file_name == normalized_project_name && length_diff <= 2 {
                            session.exact_matches.insert(idx);
                        }
                    }
                } else if matches.len() > 1 {
                    for (match_idx, (_, project_name)) in matches.iter().enumerate() {
                        let normalized_project_name = Self::normalize_name(project_name);
                        let length_diff = (normalized_file_name.len() as i32
                            - normalized_project_name.len() as i32)
                            .abs();

                        if normalized_file_name == normalized_project_name && length_diff <= 2 {
                            session.selected_matches.insert(idx, match_idx);
                            session.exact_matches.insert(idx);
                            break;
                        }
                    }
                }
            }
        }
    }
}
