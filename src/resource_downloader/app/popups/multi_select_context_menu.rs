use crate::common::prefabs::popup_window::Popup;
use crate::get_list;
use crate::get_project_versions;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::project_actions::ProjectActions;
use crate::resource_downloader::domain::{ListLnk, ProjectLnk, ResourceType};
use eframe::egui;
use egui::{Color32, Id, Ui};
use std::collections::HashSet;

#[derive(Clone)]
pub struct MultiSelectContextMenu {
    state: SharedRDState,
    list_lnk: ListLnk,
    selected: HashSet<ProjectLnk>,
}

impl MultiSelectContextMenu {
    pub fn new(state: SharedRDState, list_lnk: ListLnk, selected: HashSet<ProjectLnk>) -> Self {
        Self {
            state,
            list_lnk,
            selected,
        }
    }

    fn show_content(&self, ui: &mut Ui, open: &mut bool) {
        let has_selection = !self.selected.is_empty();
        let list_arc = get_list!(self.state, &self.list_lnk);

        let (auto_update_enabled, game_ver, loader, content_type) = {
            let list = list_arc.read();
            let content_type = list
                .get_resource_types()
                .first()
                .cloned()
                .unwrap_or(ResourceType::Mod);
            let config = list.get_resource_type_config(&content_type).unwrap();
            (
                list.get_do_updates(),
                list.get_game_version(),
                config.loader.clone(),
                content_type,
            )
        };

        let mut updates_available = Vec::new();
        if !auto_update_enabled {
            for p_lnk in &self.selected {
                let vers = get_project_versions!(
                    self.state,
                    p_lnk.clone(),
                    content_type,
                    game_ver.clone(),
                    loader.clone()
                );
                if let Ok(Some(v_list)) = vers
                    && !v_list.is_empty()
                {
                    let latest = v_list.first().unwrap();
                    let list_guard = list_arc.read();
                    if let Some(proj) = list_guard.get_project(p_lnk) {
                        if let Some(cur_v) = proj.get_version() {
                            if cur_v.artifact_hash != latest.artifact_hash {
                                updates_available.push(p_lnk.clone());
                            }
                        } else {
                            updates_available.push(p_lnk.clone());
                        }
                    }
                }
            }
        }

        let (has_clipboard, clip_is_cut, clip_list_name) = {
            let s = self.state.read();
            if let Some(c) = &s.clipboard {
                let name = s
                    .list_pool
                    .get(&c.source_list)
                    .map(|l| l.read().get_name())
                    .unwrap_or_default();
                (true, c.is_cut, name)
            } else {
                (false, false, String::new())
            }
        };

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            if has_selection {
                let copy_btn = egui::Button::new(egui::RichText::new("📋  Copy"));
                if ui.add(copy_btn).clicked() {
                    let items: Vec<_> = self.selected.iter().cloned().collect();
                    self.state
                        .write()
                        .set_clipboard(self.list_lnk.clone(), items, false);
                    *open = false;
                }

                let cut_btn = egui::Button::new(egui::RichText::new("✂  Cut"));
                if ui.add(cut_btn).clicked() {
                    let items: Vec<_> = self.selected.iter().cloned().collect();
                    self.state
                        .write()
                        .set_clipboard(self.list_lnk.clone(), items, true);
                    *open = false;
                }

                ui.separator();
            }

            if has_clipboard {
                let label = if clip_is_cut {
                    format!("📋  Paste (Move from {})", clip_list_name)
                } else {
                    format!("📋  Paste (Copy from {})", clip_list_name)
                };

                let paste_btn = egui::Button::new(egui::RichText::new(label));

                if ui.add(paste_btn).clicked() {
                    self.state.write().paste_clipboard(self.list_lnk.clone());
                    *open = false;
                }
                ui.separator();
            }
        });

        let (num_archived, num_not_archived) = {
            let list = list_arc.read();
            let mut archived = 0;
            let mut not_archived = 0;
            for p_lnk in &self.selected {
                if list.is_project_archived(p_lnk) {
                    archived += 1;
                } else {
                    not_archived += 1;
                }
            }
            (archived, not_archived)
        };

        if has_selection {
            let count = self.selected.len();
            ui.label(
                egui::RichText::new(format!("{} items selected", count))
                    .small()
                    .weak(),
            );
            ui.separator();
        }

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            if !updates_available.is_empty() {
                let update_btn = egui::Button::new(
                    egui::RichText::new(format!("🔄  Update {} Selected", updates_available.len()))
                        .color(Color32::from_rgb(0, 150, 240)),
                );
                if ui.add(update_btn).clicked() {
                    ProjectActions::update_selected_projects(
                        self.state.clone(),
                        self.list_lnk.clone(),
                        updates_available,
                    );
                    *open = false;
                }
                ui.separator();
            }

            let download_btn = egui::Button::new(
                egui::RichText::new("⬇  Download Selected").color(Color32::LIGHT_BLUE),
            );
            if ui.add(download_btn).clicked() {
                let found_hashes: HashSet<String> = self
                    .state
                    .read()
                    .found_files
                    .values()
                    .flatten()
                    .map(|(_, h)| h.clone())
                    .collect();

                ProjectActions::download_projects_latest(
                    self.state.clone(),
                    self.list_lnk.clone(),
                    self.selected.iter().cloned().collect(),
                    &found_hashes,
                );
                *open = false;
            }

            ui.separator();

            if num_not_archived > 0 {
                let archive_btn = egui::Button::new(
                    egui::RichText::new("📁  Archive Selected").color(Color32::LIGHT_YELLOW),
                );
                if ui.add(archive_btn).clicked() {
                    ProjectActions::archive_projects(
                        self.state.clone(),
                        self.list_lnk.clone(),
                        self.selected.iter().cloned().collect(),
                        true,
                    );
                    *open = false;
                }
            }

            if num_archived > 0 {
                let unarchive_btn = egui::Button::new(
                    egui::RichText::new("📂  Unarchive Selected").color(Color32::LIGHT_YELLOW),
                );
                if ui.add(unarchive_btn).clicked() {
                    ProjectActions::archive_projects(
                        self.state.clone(),
                        self.list_lnk.clone(),
                        self.selected.iter().cloned().collect(),
                        false,
                    );
                    *open = false;
                }
            }

            ui.separator();

            let delete_btn = egui::Button::new(
                egui::RichText::new("🗑  Delete Selected").color(Color32::LIGHT_RED),
            );
            if ui.add(delete_btn).clicked() {
                ProjectActions::delete_projects(
                    self.state.clone(),
                    self.list_lnk.clone(),
                    self.selected.iter().cloned().collect(),
                );
                *open = false;
            }
        });
    }
}

impl Popup for MultiSelectContextMenu {
    fn id(&self) -> Id {
        Id::new("multi_select_context_menu")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(180.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        self.show_content(ui, open);
    }
}
