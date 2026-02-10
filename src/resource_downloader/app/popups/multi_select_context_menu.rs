use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{
    ListLnk, ProjectDependencyType, ProjectLnk, RTProjectVersion, ResourceType,
};
use crate::{get_list, get_project_versions};
use eframe::egui;
use egui::{Color32, Id, Ui};
use std::collections::HashSet;
use std::path::PathBuf;

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

    #[allow(clippy::too_many_arguments)]
    fn trigger_download(
        &self,
        lnk: &ListLnk,
        p_lnk: &ProjectLnk,
        version: &RTProjectVersion,
        dir: &String,
        rt: &ResourceType,
        found_hashes: &HashSet<String>,
        triggered: &mut HashSet<ProjectLnk>,
    ) {
        if triggered.contains(p_lnk) {
            return;
        }
        triggered.insert(p_lnk.clone());

        let is_downloaded = found_hashes.contains(&version.artifact_hash);

        if !is_downloaded {
            let safe_name = {
                let list_arc = get_list!(self.state, lnk);
                let list = list_arc.read();
                list.get_project(p_lnk).unwrap().get_safe_filename()
            };

            let dest = PathBuf::from(dir).join(safe_name);

            self.state.write().download_artifact(
                &self.state,
                p_lnk.clone(),
                *rt,
                version.version_id.clone(),
                version.artifact_id.clone(),
                dest,
            );
        }

        for dep in &version.depended_on {
            if dep.dependency_type == ProjectDependencyType::Required {
                self.trigger_download(lnk, &dep.project, version, dir, rt, found_hashes, triggered);
            }
        }
    }

    fn show_content(&self, ui: &mut Ui, open: &mut bool) {
        let has_selection = !self.selected.is_empty();
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
                let copy_btn =
                    egui::Button::new(egui::RichText::new("📋  Copy").color(Color32::LIGHT_GREEN));
                if ui.add(copy_btn).clicked() {
                    let items: Vec<_> = self.selected.iter().cloned().collect();
                    self.state
                        .write()
                        .set_clipboard(self.list_lnk.clone(), items, false);
                    *open = false;
                }

                let cut_btn =
                    egui::Button::new(egui::RichText::new("✂  Cut").color(Color32::LIGHT_RED));
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

                let paste_btn =
                    egui::Button::new(egui::RichText::new(label).color(Color32::LIGHT_BLUE));

                if ui.add(paste_btn).clicked() {
                    self.state.write().paste_clipboard(self.list_lnk.clone());
                    *open = false;
                }
                ui.separator();
            }
        });

        let list_arc = get_list!(self.state, &self.list_lnk);
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
            let download_btn = egui::Button::new(
                egui::RichText::new("⬇  Download All Selected").color(Color32::LIGHT_BLUE),
            );
            if ui.add(download_btn).clicked() {
                let (ver, loader, dir, content_type) = {
                    let list = list_arc.read();
                    let rt = list
                        .get_resource_types()
                        .first()
                        .cloned()
                        .unwrap_or(ResourceType::Mod);
                    let config = list.get_resource_type_config(&rt).unwrap();
                    (
                        list.get_game_version().clone(),
                        config.loader.clone(),
                        config.download_dir.clone(),
                        rt,
                    )
                };

                let found_hashes: HashSet<String> = self
                    .state
                    .read()
                    .found_files
                    .values()
                    .flatten()
                    .map(|(_, h)| h.clone())
                    .collect();
                let mut triggered = HashSet::new();

                for p_lnk in &self.selected {
                    let versions = get_project_versions!(
                        self.state,
                        p_lnk.clone(),
                        content_type,
                        ver.clone(),
                        loader.clone()
                    );

                    if let Ok(Some(v_list)) = versions
                        && let Some(latest) = v_list.first()
                    {
                        self.trigger_download(
                            &self.list_lnk,
                            p_lnk,
                            latest,
                            &dir,
                            &content_type,
                            &found_hashes,
                            &mut triggered,
                        );
                    }
                }
                *open = false;
            }

            ui.separator();

            if num_not_archived > 0 {
                let archive_btn = egui::Button::new(
                    egui::RichText::new("📁  Archive Selected").color(Color32::LIGHT_YELLOW),
                );
                if ui.add(archive_btn).clicked() {
                    for p_lnk in &self.selected {
                        let p_lnk_clone = p_lnk.clone();
                        self.state
                            .read()
                            .list_pool
                            .mutate(&self.list_lnk, move |list| {
                                list.archive_project(&p_lnk_clone, true)
                            });
                    }
                    *open = false;
                }
            }

            if num_archived > 0 {
                let unarchive_btn = egui::Button::new(
                    egui::RichText::new("📂  Unarchive Selected").color(Color32::LIGHT_YELLOW),
                );
                if ui.add(unarchive_btn).clicked() {
                    for p_lnk in &self.selected {
                        let p_lnk_clone = p_lnk.clone();
                        self.state
                            .read()
                            .list_pool
                            .mutate(&self.list_lnk, move |list| {
                                list.archive_project(&p_lnk_clone, false)
                            });
                    }
                    *open = false;
                }
            }

            ui.separator();

            let delete_btn = egui::Button::new(
                egui::RichText::new("🗑  Delete Selected").color(Color32::LIGHT_RED),
            );
            if ui.add(delete_btn).clicked() {
                for p_lnk in &self.selected {
                    let p_lnk_clone = p_lnk.clone();
                    self.state
                        .read()
                        .list_pool
                        .mutate(&self.list_lnk, move |list| {
                            list.remove_project(&p_lnk_clone)
                        });
                }
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
