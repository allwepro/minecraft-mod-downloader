use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::dialogs::Dialogs;
use crate::resource_downloader::app::modals::list_settings_modal::ListSettingsModal;
use crate::resource_downloader::app::modals::search_modal::SearchModal;
use crate::resource_downloader::app::popups::multi_select_context_menu::MultiSelectContextMenu;
use crate::resource_downloader::app::popups::sort_popup::SortPopup;
use crate::resource_downloader::business::DownloadStatus;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::project_actions::ProjectActions;
use crate::resource_downloader::domain::{
    FilterMode, GameLoader, GameVersion, ListLnk, OrderMode, ProjectDependencyType, ProjectList,
    ProjectLnk, ResourceType, SortMode,
};
use crate::{
    clear_project_metadata, get_list, get_list_type, get_project_icon_texture, get_project_link,
    get_project_metadata, get_project_versions, get_project_versions_best,
};
use eframe::egui;
use egui::{Color32, Context, Ui};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

pub struct MainPanel {
    state: SharedRDState,
    sort_popup: SortPopup,

    rename_input_open: bool,
    rename_input: String,

    search_query: String,
    selected_projects: HashSet<ProjectLnk>,
    last_selected: Option<ProjectLnk>,
    current_list: Option<ListLnk>,
    context_menu_target: Option<(ProjectLnk, egui::Rect)>,
    should_scroll_into_view: Option<ProjectLnk>,
    expanded_depended_on: Option<ProjectLnk>,
    debug_overlays: bool,
}

impl MainPanel {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            sort_popup: SortPopup::new(state.clone()),
            rename_input_open: false,
            rename_input: String::new(),
            search_query: String::new(),
            selected_projects: HashSet::new(),
            last_selected: None,
            current_list: None,
            context_menu_target: None,
            should_scroll_into_view: None,
            expanded_depended_on: None,
            debug_overlays: false,
        }
    }

    pub fn show(&mut self, ctx: &Context, _ui: &mut Ui) {
        if ctx.input(|i| {
            i.modifiers.ctrl && i.modifiers.shift && i.modifiers.alt && i.key_pressed(egui::Key::D)
        }) {
            self.debug_overlays = !self.debug_overlays;
        }

        let (open_list_lnk, found_files_map, active_scans, pending_scroll) = {
            let mut s = self.state.write();
            (
                s.open_list.clone(),
                s.found_files.clone(),
                s.active_scans.clone(),
                s.pending_scroll.take(),
            )
        };

        if self.current_list != open_list_lnk {
            self.selected_projects.clear();
            self.current_list = open_list_lnk.clone();
        }

        if let Some((l, p)) = pending_scroll
            && Some(l) == open_list_lnk
        {
            self.should_scroll_into_view = Some(p);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.selected_projects.clear();
            }

            let lnk = match &open_list_lnk {
                Some(l) => l.clone(),
                None => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading("No list selected");
                        ui.label("Select a list from the sidebar or create a new one");
                    });
                    return;
                }
            };

            let content_type = get_list_type!(self.state, &lnk);
            let list_arc = get_list!(self.state, &lnk);

            let (
                list_name,
                ver,
                loader,
                dir,
                projects_empty,
                proj_count,
                manual_prj_count,
                show_archived,
                show_unknown_mods,
                auto_update_enabled,
            ) = {
                let list = list_arc.read();
                let rt_config = list
                    .get_resource_type_config(&content_type)
                    .expect("List without type");
                (
                    list.get_name(),
                    list.get_game_version().clone(),
                    rt_config.loader.clone(),
                    rt_config.download_dir.clone(),
                    list.manual_projects_by_type(content_type).is_empty(),
                    list.count_projects_by_type(content_type),
                    list.count_manual_projects_by_type(content_type),
                    list.is_show_archived(),
                    list.is_show_unknown_mods(),
                    list.get_do_updates(),
                )
            };

            ui.horizontal(|ui| {
                if self.rename_input_open {
                    ui.text_edit_singleline(&mut self.rename_input);
                    if ui.button("✔").clicked() {
                        ListActions::rename_list(
                            self.state.clone(),
                            lnk.clone(),
                            self.rename_input.clone(),
                        );
                        self.rename_input_open = false;
                    }
                    if ui.button("❌").clicked() {
                        self.rename_input_open = false;
                    }
                } else {
                    ui.heading(format!("{} {}", content_type.emoji(), list_name));
                    ui.add_space(1.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} (+ {}) | {} | {}",
                            manual_prj_count,
                            proj_count - manual_prj_count,
                            ver.name,
                            loader.name
                        ))
                        .small()
                        .weak(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(
                                egui::RichText::new("🗑 Delete").color(Color32::LIGHT_RED),
                            ))
                            .clicked()
                        {
                            ListActions::delete_list(self.state.clone(), lnk.clone());
                        }
                        if ui.add(egui::Button::new("✏ Rename")).clicked() {
                            self.rename_input = list_name.clone();
                            self.rename_input_open = true;
                        }
                        if ui.add(egui::Button::new("👥 Duplicate")).clicked() {
                            ListActions::duplicate_list(self.state.clone(), lnk.clone());
                        }
                        if ui.add(egui::Button::new("📂 Open Folder")).clicked() {
                            ListActions::open_folder(self.state.clone(), lnk.clone());
                        }

                        if ui.add(egui::Button::new("📤 Export")).clicked()
                            && let Some(path) = Dialogs::save_export_list_file(
                                &list_name,
                                content_type == ResourceType::Mod,
                            )
                        {
                            let ext = path.extension().and_then(|s| s.to_str());
                            if ext == Some("toml") || ext == Some("mmd") {
                                ListActions::export_list(self.state.clone(), lnk.clone(), path);
                            } else if content_type == ResourceType::Mod {
                                ListActions::export_legacy_list(
                                    self.state.clone(),
                                    lnk.clone(),
                                    path,
                                    ver.clone(),
                                    loader.clone(),
                                );
                            }
                        }

                        if ui.button("⚙ List Settings").clicked() {
                            let sm = ListSettingsModal::new(self.state.clone(), lnk.clone());
                            self.state.read().submit_modal(Box::new(sm));
                        }
                    });
                }
            });

            ui.separator();

            let combined_found_files: Vec<(PathBuf, String)> =
                found_files_map.values().flatten().cloned().collect();
            let found_hashes: HashSet<String> = combined_found_files
                .iter()
                .map(|(_, h)| h.clone())
                .collect();

            let filtered =
                self.get_filtered_projects(&list_arc, &content_type, &found_hashes, &ver, &loader);

            let mut any_updates_available = false;
            if !auto_update_enabled {
                for p_lnk in &filtered {
                    let vers = get_project_versions!(
                        self.state,
                        p_lnk.clone(),
                        content_type,
                        ver.clone(),
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
                                    any_updates_available = true;
                                    break;
                                }
                            } else {
                                any_updates_available = true;
                                break;
                            }
                        }
                    }
                }
            }

            let row_height = 32.0;
            let full_rect = ui.available_rect_before_wrap();
            let full_rect =
                egui::Rect::from_min_size(full_rect.min, egui::vec2(full_rect.width(), row_height));

            ui.allocate_rect(full_rect, egui::Sense::hover());

            let left_rect = ui
                .scope_builder(egui::UiBuilder::new().max_rect(full_rect), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new(format!(
                                    "➕ Add {}",
                                    content_type.display_name()
                                ))
                                .color(Color32::LIGHT_GREEN),
                            )
                            .clicked()
                        {
                            let sm = SearchModal::new(
                                self.state.clone(),
                                lnk.clone(),
                                content_type,
                                ver.clone(),
                                loader.clone(),
                            );
                            self.state.read().submit_modal(Box::new(sm));
                        }
                    })
                    .response
                    .rect
                })
                .inner;

            let right_rect = ui
                .scope_builder(egui::UiBuilder::new().max_rect(full_rect), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let missing: Vec<ProjectLnk> = {
                            filtered
                                .iter()
                                .filter(|p| {
                                    let list = list_arc.read();
                                    if let Some(proj) = list.get_project(p) {
                                        let is_downloaded = proj.get_version().is_some_and(|v| {
                                            found_hashes.contains(&v.artifact_hash)
                                        });
                                        !list.is_project_archived(p) && !is_downloaded
                                    } else {
                                        false
                                    }
                                })
                                .cloned()
                                .collect()
                        };

                        let mut combined_res: Option<egui::Response>;

                        let res1 = ui.add_enabled(
                            !missing.is_empty(),
                            egui::Button::new(
                                egui::RichText::new("⬇ Download All").color(Color32::LIGHT_BLUE),
                            ),
                        );

                        if res1.clicked() {
                            ProjectActions::download_projects_latest(
                                self.state.clone(),
                                lnk.clone(),
                                missing,
                                &found_hashes,
                            );
                        }
                        combined_res = Some(res1);

                        if !auto_update_enabled && any_updates_available {
                            let res2 = ui
                                .button(
                                    egui::RichText::new("🔄 Update All")
                                        .color(Color32::from_rgb(0, 150, 240)),
                                )
                                .on_hover_text("Update all projects to their latest versions");

                            if res2.clicked() {
                                ProjectActions::update_all_projects(
                                    self.state.clone(),
                                    lnk.clone(),
                                );
                            }

                            if let Some(res) = combined_res {
                                combined_res = Some(res.union(res2));
                            } else {
                                combined_res = Some(res2);
                            }
                        }

                        combined_res.map(|r| r.rect).unwrap_or(egui::Rect::NOTHING)
                    })
                    .inner
                })
                .inner;

            let mut measure_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(ui.available_rect_before_wrap())
                    .layout(egui::Layout::left_to_right(egui::Align::Center))
                    .ui_stack_info(egui::UiStackInfo::default()),
            );
            measure_ui.set_clip_rect(egui::Rect::ZERO);

            let measure_res = measure_ui.scope(|ui| {
                self.render_header_controls(ui, &list_arc, &content_type, true);
            });
            let controls_width = measure_res.response.rect.width();

            let left_boundary = if left_rect.is_positive() {
                left_rect.max.x + 12.0
            } else {
                full_rect.min.x
            };
            let right_boundary = if right_rect.is_positive() {
                right_rect.min.x - 12.0
            } else {
                full_rect.max.x
            };

            let available_center = (left_boundary + right_boundary) / 2.0;
            let ideal_center_x = if (available_center - full_rect.center().x).abs() < 50.0 {
                full_rect.center().x
            } else {
                available_center
            };

            let ideal_left = ideal_center_x - (controls_width / 2.0);
            let mut final_left = ideal_left.max(left_boundary);

            if final_left + controls_width > right_boundary {
                final_left = right_boundary - controls_width;
                if final_left < left_boundary {
                    final_left = left_boundary;
                }
            }

            let final_width = controls_width.min(available_center);

            let center_rect = egui::Rect::from_min_size(
                egui::pos2(final_left, full_rect.min.y),
                egui::vec2(final_width, full_rect.height()),
            );

            if self.debug_overlays {
                let painter = ui.ctx().debug_painter();
                painter.rect_filled(
                    full_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 0, 255, 20),
                ); // Blue overlay for header
                if left_rect.is_positive() {
                    painter.rect_filled(
                        left_rect,
                        0.0,
                        Color32::from_rgba_unmultiplied(0, 255, 0, 40),
                    ); // Green overlay for left buttons
                }
                if right_rect.is_positive() {
                    painter.rect_filled(
                        right_rect,
                        0.0,
                        Color32::from_rgba_unmultiplied(255, 0, 0, 40),
                    ); // Red overlay for right buttons
                }
                painter.rect_filled(
                    center_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(255, 255, 0, 40),
                ); // Yellow overlay for center controls
            }

            ui.scope_builder(egui::UiBuilder::new().max_rect(center_rect), |ui| {
                ui.centered_and_justified(|ui| {
                    self.render_header_controls(ui, &list_arc, &content_type, false);
                });
            });

            ui.add_space(4.0);

            if projects_empty {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("No items in this list");

                    let (has_clipboard, clip_list_name) = {
                        let s = self.state.read();
                        if let Some(c) = &s.clipboard {
                            let name = s
                                .list_pool
                                .get(&c.source_list)
                                .map(|l| l.read().get_name())
                                .unwrap_or_default();
                            (true, name)
                        } else {
                            (false, String::new())
                        }
                    };

                    if has_clipboard {
                        ui.add_space(10.0);
                        if ui
                            .add(egui::Button::new(egui::RichText::new(format!(
                                "📋 Paste from {}",
                                clip_list_name
                            ))))
                            .clicked()
                        {
                            self.state.write().paste_clipboard(lnk.clone());
                        }
                    }
                });
            } else {
                let filtered = self.get_filtered_projects(
                    &list_arc,
                    &content_type,
                    &found_hashes,
                    &ver,
                    &loader,
                );
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let (active, archived): (Vec<_>, Vec<_>) = filtered
                        .into_iter()
                        .partition(|p| !list_arc.read().is_project_archived(p));

                    for (idx, p_lnk) in active.iter().enumerate() {
                        self.render_project_entry(
                            ui,
                            &lnk,
                            &list_arc,
                            p_lnk,
                            &content_type,
                            &ver,
                            &loader,
                            &combined_found_files,
                            &found_hashes,
                            &dir,
                            false,
                            &active_scans,
                            &active,
                            idx,
                        );
                    }

                    let archived_count = archived.len();
                    if archived_count > 0 {
                        ui.add_space(8.0);
                        ui.separator();
                        if ui
                            .button(egui::RichText::new(format!(
                                "{} Archived Projects ({})",
                                if show_archived { "🔽" } else { "▶" },
                                archived_count
                            )))
                            .clicked()
                        {
                            list_arc.write().set_show_archived(!show_archived);
                            self.state.read().list_pool.save(&lnk);
                        }

                        if show_archived {
                            for (idx, p_lnk) in archived.iter().enumerate() {
                                self.render_project_entry(
                                    ui,
                                    &lnk,
                                    &list_arc,
                                    p_lnk,
                                    &content_type,
                                    &ver,
                                    &loader,
                                    &combined_found_files,
                                    &found_hashes,
                                    &dir,
                                    false,
                                    &active_scans,
                                    &archived,
                                    idx,
                                );
                            }
                        }
                    }

                    let search_lower = self.search_query.to_lowercase();
                    let unknown_files = self.get_unknown_files(
                        &list_arc.read(),
                        &content_type,
                        &combined_found_files,
                        &search_lower,
                    );
                    let unknown_count = unknown_files.len();
                    if unknown_count > 0 {
                        ui.add_space(8.0);
                        ui.separator();
                        if ui
                            .button(format!(
                                "{} Unknown Projects ({})",
                                if show_unknown_mods { "🔽" } else { "▶" },
                                unknown_count
                            ))
                            .clicked()
                        {
                            list_arc.write().set_show_unknown_mods(!show_unknown_mods);
                            self.state.read().list_pool.save(&lnk);
                        }

                        if show_unknown_mods {
                            for (path, _hash) in unknown_files {
                                let filename = path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                self.render_unknown_entry(ui, path, &filename);
                            }
                        }
                    }
                });
            }
        });

        if let Some((_target_project, rect)) = &self.context_menu_target {
            let pm = self.state.read().popup_manager.clone();
            let menu_id = egui::Id::new("multi_select_context_menu");

            if pm.is_open(menu_id) {
                if let Some(open_lnk) = &open_list_lnk {
                    let menu = MultiSelectContextMenu::new(
                        self.state.clone(),
                        open_lnk.clone(),
                        self.selected_projects.clone(),
                    );
                    pm.register_interaction_area(menu_id, *rect);
                    pm.request_show(Box::new(menu), *rect);
                }
            } else {
                self.context_menu_target = None;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_project_entry(
        &mut self,
        ui: &mut Ui,
        lnk: &ListLnk,
        list_arc: &Arc<RwLock<ProjectList>>,
        p_lnk: &ProjectLnk,
        rt: &ResourceType,
        g_ver: &GameVersion,
        g_ld: &GameLoader,
        combined_found_files: &[(PathBuf, String)],
        found_hashes: &HashSet<String>,
        dir: &String,
        is_dependency: bool,
        active_scans: &HashSet<PathBuf>,
        ordered_list: &[ProjectLnk],
        current_idx: usize,
    ) {
        let is_overruled = {
            let list = list_arc.read();
            list.get_project(p_lnk)
                .map(|p| p.is_compatibility_overruled())
                .unwrap_or(false)
        };

        let (metadata, mut versions) = {
            let meta = get_project_metadata!(self.state, p_lnk.clone(), *rt);
            let vers =
                get_project_versions!(self.state, p_lnk.clone(), *rt, g_ver.clone(), g_ld.clone());
            (meta, vers)
        };

        let compatibility = if let Ok(Some(vers)) = &versions {
            Some(!vers.is_empty())
        } else {
            None
        };

        if matches!(compatibility, Some(false))
            && is_overruled
            && let Ok(Some(best_vers)) = get_project_versions_best!(
                self.state,
                p_lnk.clone(),
                *rt,
                g_ver.clone(),
                g_ld.clone()
            )
        {
            versions = Ok(Some(best_vers));
        }

        let auto_update_enabled = list_arc.read().get_do_updates();

        if let (Ok(Some(meta)), Ok(Some(vers))) = (&metadata, &versions)
            && !vers.is_empty()
        {
            let latest = vers.first().unwrap();
            let mut list = list_arc.write();
            if let Some(project) = list.get_project_mut(p_lnk) {
                project.update_cache(meta.clone());

                let is_missing_version = project.get_version().is_none();
                let is_new_version_available = project
                    .get_version()
                    .as_ref()
                    .is_some_and(|v| v.version_id != latest.version_id);

                if is_missing_version || (auto_update_enabled && is_new_version_available) {
                    drop(list);
                    self.state.read().list_pool.select_version(
                        &list_arc.read().get_lnk(),
                        p_lnk.clone(),
                        latest.version_id.clone(),
                    );
                }
            }
        }

        let (
            name,
            author,
            version_id,
            is_archived,
            cur_hash,
            has_dependents,
            depended_on,
            filename,
        ) = {
            let p = list_arc.read();
            let Some(proj) = p.get_project(p_lnk) else {
                return;
            };
            (
                proj.get_name(),
                proj.get_author(),
                proj.get_version_id().map(|s| s.to_string()),
                p.is_project_archived(p_lnk),
                proj.get_version().map(|v| v.artifact_hash.clone()),
                proj.has_dependents(),
                proj.get_version().map(|v| v.get_depended_ons().to_vec()),
                proj.get_safe_filename(),
            )
        };

        let file_on_disk = combined_found_files.iter().find(|(path, _)| {
            path.file_name().is_some_and(|n| {
                n == filename.as_str() || n == format!("{filename}.archive").as_str()
            })
        });
        let is_file_present = file_on_disk.is_some();
        let disk_hash = file_on_disk.map(|(_, h)| h.clone());

        let has_failed = metadata.is_err() || versions.is_err();
        let is_scanning_this_dir = active_scans.contains(&PathBuf::from(dir));
        let has_loaded_files = !is_scanning_this_dir;
        let is_downloaded = disk_hash.is_some() && disk_hash == cur_hash;

        let mut is_updatable = false;
        let mut is_version_outdated = false;
        let mut latest_version_name = "⏳".to_string();
        let mut current_version_name = None;

        if let Ok(Some(vers)) = &versions
            && !vers.is_empty()
        {
            let latest = vers.first().unwrap();
            latest_version_name = latest.version_name.clone();

            if let Some(vid) = &version_id {
                current_version_name = vers
                    .iter()
                    .find(|v| &v.version_id == vid)
                    .map(|v| v.version_name.clone());

                is_version_outdated = vid != &latest.version_id;
            }

            is_updatable = is_file_present && disk_hash.as_ref() != Some(&latest.artifact_hash);
        }

        let mut display_version = if let Some(cur) = current_version_name {
            if is_version_outdated {
                format!("v{} -> v{}", cur, latest_version_name)
            } else {
                format!("v{}", cur)
            }
        } else {
            format!("v{}", latest_version_name)
        };

        if display_version.contains("vv") {
            display_version = display_version.replace("vv", "v");
        }

        let dl_status = self
            .state
            .read()
            .download_status
            .get(p_lnk)
            .cloned()
            .unwrap_or((DownloadStatus::Idle, 0.0));
        let should_scroll = self.should_scroll_into_view.as_ref() == Some(p_lnk);
        let is_selected = self.selected_projects.contains(p_lnk);

        let mut frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(egui::Stroke::new(
                1.0,
                if is_selected {
                    Color32::from_rgb(100, 150, 255)
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke.color
                },
            ))
            .corner_radius(6.0)
            .inner_margin(8.0);

        if is_selected {
            frame = frame.fill(ui.visuals().selection.bg_fill.linear_multiply(0.1));
        }

        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let icon_size = if is_dependency { 24.0 } else { 32.0 };
                    if let Some(tex) = get_project_icon_texture!(self.state, p_lnk) {
                        ui.add(
                            egui::Image::from_texture(&tex)
                                .fit_to_exact_size(egui::vec2(icon_size, icon_size)),
                        );
                    } else {
                        ui.add_sized([icon_size, icon_size], egui::Spinner::new());
                    }

                    ui.add_space(4.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !is_dependency {
                            if ui
                                .add_enabled(
                                    !has_dependents,
                                    egui::Button::new(
                                        egui::RichText::new("🗑").color(Color32::LIGHT_RED),
                                    ),
                                )
                                .on_hover_text("Remove from list")
                                .clicked()
                            {
                                ProjectActions::delete_projects(
                                    self.state.clone(),
                                    lnk.clone(),
                                    vec![p_lnk.clone()],
                                );
                            }

                            let archive_label = if is_archived {
                                "📂 Unarchive"
                            } else {
                                "📁 Archive"
                            };
                            if ui
                                .add_enabled(
                                    !has_dependents,
                                    egui::Button::new(
                                        egui::RichText::new(archive_label)
                                            .color(Color32::LIGHT_YELLOW),
                                    ),
                                )
                                .clicked()
                            {
                                ProjectActions::archive_projects(
                                    self.state.clone(),
                                    lnk.clone(),
                                    vec![p_lnk.clone()],
                                    !is_archived,
                                );

                                if is_archived {
                                    self.should_scroll_into_view = Some(p_lnk.clone());
                                }
                            }
                        }

                        if !is_archived {
                            match dl_status.0 {
                                DownloadStatus::Downloading | DownloadStatus::Queued => {
                                    ui.add(
                                        egui::ProgressBar::new(dl_status.1)
                                            .text(format!("{:.0}%", dl_status.1 * 100.0))
                                            .desired_width(80.0),
                                    );
                                }
                                _ => {
                                    let btn_label = "Download";
                                    let can_dl =
                                        matches!(compatibility, Some(true)) || is_overruled;
                                    let ui_enabled = can_dl && !is_downloaded && has_loaded_files;

                                    let latest_version = if let Ok(Some(v_list)) = &versions {
                                        v_list.first()
                                    } else {
                                        None
                                    };

                                    let btn = ui.add_enabled(
                                        ui_enabled && latest_version.is_some(),
                                        egui::Button::new(
                                            egui::RichText::new(btn_label)
                                                .color(Color32::LIGHT_BLUE),
                                        ),
                                    );

                                    if btn.clicked()
                                        && let Some(v) = latest_version
                                    {
                                        ProjectActions::download_project_specific(
                                            self.state.clone(),
                                            lnk.clone(),
                                            p_lnk.clone(),
                                            v,
                                            found_hashes,
                                        );
                                    }

                                    if is_downloaded {
                                        ui.label("✅");
                                    }
                                }
                            }
                        }

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    let mut name_rich = egui::RichText::new(&name).strong();
                                    if is_archived {
                                        name_rich = name_rich.weak();
                                    }
                                    ui.hyperlink_to(
                                        name_rich,
                                        get_project_link!(self.state, p_lnk, rt),
                                    );
                                });

                                if has_failed {
                                    if ui
                                        .button(
                                            egui::RichText::new("⚠ Failed to load")
                                                .color(Color32::YELLOW),
                                        )
                                        .clicked()
                                    {
                                        clear_project_metadata!(self.state, p_lnk.clone(), *rt);
                                    }
                                } else {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{display_version} by {author}"
                                        ))
                                        .small()
                                        .weak(),
                                    );
                                }

                                if !is_dependency {
                                    ui.horizontal(|ui| {
                                        if let Some(depended_ons) = depended_on.clone() {
                                            let required_deps: Vec<_> = depended_ons
                                                .iter()
                                                .filter(|dep| {
                                                    dep.dependency_type
                                                        == ProjectDependencyType::Required
                                                })
                                                .collect();

                                            if !required_deps.is_empty() {
                                                let is_expanded = self
                                                    .expanded_depended_on
                                                    .as_ref()
                                                    .is_some_and(|id| id == p_lnk);

                                                let badge_text = format!(
                                                    "+{} Dependencies",
                                                    required_deps.len()
                                                );
                                                let mut badge_color =
                                                    Color32::from_rgb(100, 150, 200);
                                                if is_expanded {
                                                    badge_color = Color32::from_rgb(150, 200, 255);
                                                }

                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new(badge_text)
                                                                .color(badge_color),
                                                        )
                                                        .small(),
                                                    )
                                                    .clicked()
                                                {
                                                    if is_expanded {
                                                        self.expanded_depended_on = None;
                                                    } else {
                                                        self.expanded_depended_on =
                                                            Some(p_lnk.clone());
                                                    }
                                                }
                                                ui.add_space(3.0);
                                            }
                                        }

                                        if is_updatable {
                                            ui.colored_label(
                                                Color32::from_rgb(0, 150, 240),
                                                "🔄 Update Available",
                                            );
                                        }
                                        if has_loaded_files
                                            && !is_archived
                                            && !is_downloaded
                                            && !is_file_present
                                            && matches!(compatibility, Some(true))
                                        {
                                            ui.colored_label(Color32::GOLD, "📁 Missing");
                                        }

                                        if is_overruled {
                                            ui.colored_label(
                                                Color32::from_rgb(255, 165, 0),
                                                "⚠ Incompatible Overruled",
                                            );
                                            if ui.small_button("🔓 Revoke").clicked() {
                                                let p_lnk_clone = p_lnk.clone();
                                                self.state.read().list_pool.mutate(
                                                    lnk,
                                                    move |list| {
                                                        list.set_compatibility_overruled(
                                                            &p_lnk_clone,
                                                            false,
                                                        )
                                                    },
                                                );
                                            }
                                        } else if matches!(compatibility, Some(false)) {
                                            ui.colored_label(Color32::RED, "❌ Incompatible");
                                            if ui.small_button("🔒 Overrule").clicked() {
                                                let p_lnk_clone = p_lnk.clone();
                                                self.state.read().list_pool.mutate(
                                                    lnk,
                                                    move |list| {
                                                        list.set_compatibility_overruled(
                                                            &p_lnk_clone,
                                                            true,
                                                        )
                                                    },
                                                );
                                            }
                                        }
                                    });
                                }
                            });
                        });
                    });
                });
            })
            .response;

        if !is_dependency {
            let is_button_clicked = ui.ctx().is_using_pointer();

            let primary_clicked = response.hovered()
                && ui.input(|i| i.pointer.primary_clicked())
                && !is_button_clicked;

            let secondary_clicked = response.hovered()
                && ui.input(|i| i.pointer.secondary_clicked())
                && !is_button_clicked;

            if primary_clicked {
                let modifiers = ui.input(|i| i.modifiers);
                if modifiers.shift
                    && let Some(last_id) = self.last_selected.as_ref()
                {
                    if let Some(last_idx) = ordered_list.iter().position(|id| id == last_id) {
                        let start = last_idx.min(current_idx);
                        let end = last_idx.max(current_idx);
                        for item in ordered_list.iter().take(end + 1).skip(start) {
                            self.selected_projects.insert(item.clone());
                        }
                    }
                } else if modifiers.command {
                    if is_selected {
                        self.selected_projects.remove(p_lnk);
                    } else {
                        self.selected_projects.insert(p_lnk.clone());
                        self.last_selected = Some(p_lnk.clone());
                    }
                } else {
                    self.selected_projects.clear();
                    self.selected_projects.insert(p_lnk.clone());
                    self.last_selected = Some(p_lnk.clone());
                }
            }

            if secondary_clicked {
                if !is_selected {
                    self.selected_projects.clear();
                    self.selected_projects.insert(p_lnk.clone());
                    self.last_selected = Some(p_lnk.clone());
                }
                self.context_menu_target = Some((p_lnk.clone(), response.rect));
                let menu_id = egui::Id::new("multi_select_context_menu");
                self.state.read().popup_manager.toggle(menu_id);
            }

            if let Some((target_lnk, _)) = &self.context_menu_target
                && target_lnk == p_lnk
            {
                self.context_menu_target = Some((p_lnk.clone(), response.rect));
            }

            if let Some(ref expanded_id) = self.expanded_depended_on.clone()
                && expanded_id == p_lnk
                && let Some(depended_ons) = depended_on
            {
                let required_deps: Vec<_> = depended_ons
                    .iter()
                    .filter(|dep| dep.dependency_type == ProjectDependencyType::Required)
                    .collect();

                if !required_deps.is_empty() {
                    ui.indent("dep_indent", |ui| {
                        ui.add_space(4.0);
                        for dep in &required_deps {
                            self.render_project_entry(
                                ui,
                                lnk,
                                list_arc,
                                &dep.project,
                                rt,
                                g_ver,
                                g_ld,
                                combined_found_files,
                                found_hashes,
                                dir,
                                true,
                                active_scans,
                                &[],
                                0,
                            );
                        }
                    });
                }
            }

            if should_scroll {
                response.scroll_to_me(Some(egui::Align::Center));
                self.should_scroll_into_view = None;
            }
        }

        ui.add_space(4.0);
    }

    fn render_header_controls(
        &mut self,
        ui: &mut Ui,
        list_arc: &Arc<RwLock<ProjectList>>,
        content_type: &ResourceType,
        is_measurement: bool,
    ) {
        ui.horizontal(|ui| {
            let is_loading = !self.state.read().active_scans.is_empty();

            ui.add_enabled_ui(!is_loading, |ui| {
                let button_res = if is_loading {
                    ui.add_sized([28.0, 24.0], egui::Spinner::new())
                        .on_hover_text("Loading files...")
                } else {
                    ui.button("🔄").on_hover_text(
                        "Refresh files from disk (Shift+Click to recalculate dependencies)",
                    )
                };

                if button_res.clicked() && !is_measurement {
                    if ui.input(|i| i.modifiers.shift) {
                        ListActions::refresh_dependencies(
                            self.state.clone(),
                            list_arc.read().get_lnk(),
                        );
                    } else {
                        {
                            let mut state = self.state.write();
                            state.request_full_refresh();
                        }

                        let list = list_arc.read();
                        for rt in list.get_resource_types() {
                            if let Some(tc) = list.get_resource_type_config(&rt) {
                                self.state.write().find_files(
                                    tc.download_dir.clone().into(),
                                    rt.file_extension(),
                                );
                            }
                        }
                    }
                }
            });

            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text(format!("🔍 Search {}s...", content_type.display_name()))
                    .desired_width(200.0),
            );
            if ui
                .add_enabled(!self.search_query.is_empty(), egui::Button::new("❌"))
                .clicked()
                && !is_measurement
            {
                self.search_query.clear();
            }

            {
                let sort_id = self.sort_popup.id();
                let sort_settings = list_arc.read().get_sort_settings();
                let sort_btn = ui.button(match sort_settings.order_mode {
                    OrderMode::Ascending => "⬇ Sort",
                    OrderMode::Descending => "⬆ Sort",
                });

                if !is_measurement {
                    self.state
                        .read()
                        .popup_manager
                        .register_interaction_area(sort_id, sort_btn.rect);
                    if sort_btn.clicked() {
                        self.state.read().popup_manager.toggle(sort_id);
                    }
                    self.state
                        .read()
                        .popup_manager
                        .request_show(Box::new(self.sort_popup.clone()), sort_btn.rect);
                }
            }
        });
    }

    fn get_filtered_projects(
        &self,
        list_arc: &Arc<RwLock<ProjectList>>,
        rt: &ResourceType,
        hashes: &HashSet<String>,
        current_ver: &GameVersion,
        current_loader: &GameLoader,
    ) -> Vec<ProjectLnk> {
        let (mut candidates, settings) = {
            let list = list_arc.read();
            let mods = list
                .manual_projects_by_type(*rt)
                .into_iter()
                .map(|p| {
                    (
                        p.get_lnk().clone(),
                        p.get_name().clone(),
                        p.get_author().clone(),
                        p.added_at,
                        p.get_version().map(|v| v.artifact_hash.clone()),
                    )
                })
                .collect::<Vec<_>>();
            (mods, list.get_sort_settings())
        };

        let query = self.search_query.to_lowercase();

        candidates.retain(|(lnk, name, author, _added_at, artifact_hash)| {
            let matches_query = query.is_empty()
                || name.to_lowercase().contains(&query)
                || author.to_lowercase().contains(&query);

            if !matches_query {
                return false;
            }

            let is_downloaded = artifact_hash.as_ref().is_some_and(|h| hashes.contains(h));

            match settings.filter_mode {
                FilterMode::All => true,
                FilterMode::MissingOnly => {
                    let is_archived = list_arc.read().is_project_archived(lnk);
                    !is_downloaded && !is_archived
                }
                FilterMode::CompatibleOnly => {
                    let vers_res = get_project_versions!(
                        self.state,
                        lnk.clone(),
                        *rt,
                        current_ver.clone(),
                        current_loader.clone()
                    );
                    if let Ok(Some(vers)) = vers_res {
                        !vers.is_empty()
                    } else {
                        true
                    }
                }
                FilterMode::IncompatibleOnly => {
                    let vers_res = get_project_versions!(
                        self.state,
                        lnk.clone(),
                        *rt,
                        current_ver.clone(),
                        current_loader.clone()
                    );
                    if let Ok(Some(vers)) = vers_res {
                        vers.is_empty()
                    } else {
                        false
                    }
                }
            }
        });

        candidates.sort_by(|a, b| {
            let res = match settings.sort_mode {
                SortMode::Name => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
                SortMode::DateAdded => a.3.cmp(&b.3),
            };

            if settings.order_mode == OrderMode::Descending {
                res.reverse()
            } else {
                res
            }
        });

        candidates.into_iter().map(|(lnk, ..)| lnk).collect()
    }

    fn get_unknown_files(
        &self,
        list: &ProjectList,
        rt: &ResourceType,
        combined_found_files: &[(PathBuf, String)],
        query: &str,
    ) -> Vec<(PathBuf, String)> {
        let known_filenames: HashSet<String> = list
            .projects_by_type(*rt)
            .iter()
            .map(|p| p.get_safe_filename())
            .collect();

        combined_found_files
            .iter()
            .filter(|(path, _hash)| {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let name_lower = name.to_lowercase();

                let is_known_name = known_filenames.contains(&name);
                let matches_query = query.is_empty() || name_lower.contains(query);

                !is_known_name && matches_query
            })
            .cloned()
            .collect()
    }

    fn render_unknown_entry(&self, ui: &mut Ui, path: PathBuf, filename: &str) {
        ui.horizontal(|ui| {
            ui.add_sized([32.0, 32.0], egui::Label::new("❓"));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(filename).weak());
                ui.label(egui::RichText::new("No metadata available").small().weak());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑").clicked() {
                    self.state.write().delete_artifact(
                        path.parent().unwrap().to_path_buf(),
                        filename.to_string(),
                    );
                }
            });
        });
        ui.separator();
    }
}
