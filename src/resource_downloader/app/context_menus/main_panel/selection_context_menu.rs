use crate::common::ui::ash_ui::AshUi;
use crate::common::ui::structs::popup_window::Popup;
use crate::get_list;
use crate::get_project_versions;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::project_actions::ProjectActions;
use crate::resource_downloader::domain::{ListLnk, ProjectLnk, ResourceType};
use eframe::egui;
use egui::{Color32, Id, Ui};
use std::collections::HashSet;

#[derive(Clone)]
pub struct SelectionContextMenu {
    state: SharedRDState,
    list_lnk: ListLnk,
    selected: HashSet<ProjectLnk>,
}

impl SelectionContextMenu {
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

        let (has_clipboard, clip_is_cut, clip_list_name, clip_resource_type) = {
            let s = self.state.read();
            if let Some(c) = &s.clipboard {
                let name = s
                    .list_pool
                    .get(&c.source_list)
                    .map(|l| l.read().get_name())
                    .unwrap_or_default();
                let rt = ListActions::get_list_resource_type(&self.state, &c.source_list);
                (true, c.is_cut, name, Some(rt))
            } else {
                (false, false, String::new(), None)
            }
        };

        ui.ash_context_menu(|ui| {
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

                    let target_rt =
                        ListActions::get_list_resource_type(&self.state, &self.list_lnk);
                    let types_match = clip_resource_type.map(|rt| rt == target_rt).unwrap_or(true);

                    let paste_btn = egui::Button::new(egui::RichText::new(label));
                    let mut response = ui.add_enabled(types_match, paste_btn);

                    if !types_match {
                        response =
                            response.on_disabled_hover_text("Cannot paste: wrong resource type");
                    }

                    if response.clicked() {
                        self.state.write().paste_clipboard(self.list_lnk.clone());
                        *open = false;
                    }
                    ui.separator();
                }
            });
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

        let blocking_dependents: Vec<(String, Vec<String>)> = self
            .selected
            .iter()
            .filter_map(|p_lnk| {
                let list = list_arc.read();
                let proj = list.get_project(p_lnk)?;
                if proj.has_dependents() {
                    let deps = proj
                        .get_dependents()
                        .iter()
                        .filter_map(|d| list.get_project(d).map(|dp| dp.get_name()))
                        .collect();
                    Some((proj.get_name(), deps))
                } else {
                    None
                }
            })
            .collect();

        let offline_mode = self.state.read().offline_mode;

        ui.ash_context_menu(|ui| {
            let count = self.selected.len();
            if has_selection && count > 1 {
                ui.label(
                    egui::RichText::new(format!("{} items selected", count))
                        .small()
                        .weak(),
                );
            }

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                if !updates_available.is_empty() {
                    let update_btn = egui::Button::new(
                        egui::RichText::new(format!(
                            "🔄  Update {} Selected",
                            updates_available.len()
                        ))
                        .color(if offline_mode {
                            Color32::GRAY
                        } else {
                            Color32::from_rgb(0, 150, 240)
                        }),
                    );

                    let mut res = ui.add_enabled(!offline_mode, update_btn);
                    if offline_mode {
                        res = res.on_disabled_hover_text("Disabled in offline mode");
                    }

                    if res.clicked() {
                        ProjectActions::update_version_for_projects(
                            self.state.clone(),
                            self.list_lnk.clone(),
                            updates_available,
                        );
                        *open = false;
                    }
                    ui.separator();
                }

                let download_btn = egui::Button::new(
                    egui::RichText::new("⬇  Download Selected").color(if offline_mode {
                        Color32::GRAY
                    } else {
                        Color32::LIGHT_BLUE
                    }),
                );

                let mut res = ui.add_enabled(!offline_mode, download_btn);
                if offline_mode {
                    res = res.on_disabled_hover_text("Disabled in offline mode");
                }

                if res.clicked() {
                    let found_hashes: HashSet<String> = self
                        .state
                        .read()
                        .found_files
                        .values()
                        .flatten()
                        .map(|(_, h)| h.clone())
                        .collect();

                    ProjectActions::download_projects(
                        self.state.clone(),
                        self.list_lnk.clone(),
                        self.selected.iter().map(|p| (p.clone(), None)).collect(),
                        &found_hashes,
                    );
                    *open = false;
                }

                let mut blocking_tooltip = String::new();
                if !blocking_dependents.is_empty() {
                    blocking_tooltip =
                        "Following projects have dependents and cannot be moved/removed:\n"
                            .to_string();
                    for (name, deps) in &blocking_dependents {
                        blocking_tooltip.push_str(&format!(
                            "• {}: required by {}\n",
                            name,
                            deps.join(", ")
                        ));
                    }
                }

                if num_not_archived > 0 {
                    let enabled = blocking_dependents.len() < self.selected.len();
                    let btn_res = ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(
                                egui::RichText::new("📁  Archive Selected")
                                    .color(Color32::LIGHT_YELLOW),
                            ),
                        )
                        .on_disabled_hover_text(&blocking_tooltip)
                        .on_hover_text(&blocking_tooltip);

                    if btn_res.clicked() {
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
                    let enabled = blocking_dependents.len() < self.selected.len();
                    let btn_res = ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(
                                egui::RichText::new("📂  Unarchive Selected")
                                    .color(Color32::LIGHT_YELLOW),
                            ),
                        )
                        .on_disabled_hover_text(&blocking_tooltip)
                        .on_hover_text(&blocking_tooltip);

                    if btn_res.clicked() {
                        ProjectActions::archive_projects(
                            self.state.clone(),
                            self.list_lnk.clone(),
                            self.selected.iter().cloned().collect(),
                            false,
                        );
                        *open = false;
                    }
                }

                let enabled = blocking_dependents.len() < self.selected.len();
                let btn_res = ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(
                            egui::RichText::new("🗑  Delete Selected").color(Color32::LIGHT_RED),
                        ),
                    )
                    .on_disabled_hover_text(&blocking_tooltip)
                    .on_hover_text(&blocking_tooltip);

                if btn_res.clicked() {
                    ProjectActions::remove_projects(
                        self.state.clone(),
                        self.list_lnk.clone(),
                        self.selected.iter().cloned().collect(),
                    );
                    *open = false;
                }
            });
        });
    }
}

impl Popup for SelectionContextMenu {
    fn id(&self) -> Id {
        Id::new("multi_select_context_menu")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(180.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        self.show_content(ui, open);
    }
}
