use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::dialogs::Dialogs;
use crate::resource_downloader::app::modals::list_settings_modal::ListSettingsModal;
use crate::resource_downloader::app::modals::search_modal::SearchModal;
use crate::resource_downloader::app::popups::sort_popup::{
    FilterMode, OrderMode, SortMode, SortPopup,
};
use crate::resource_downloader::business::DownloadStatus;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{
    GameLoader, GameVersion, ProjectDependencyType, ProjectList, ProjectLnk, RTProjectVersion,
    ResourceType,
};
use crate::{
    clear_project_metadata, get_list, get_list_type, get_project_icon_texture, get_project_link,
    get_project_metadata, get_project_versions,
};
use eframe::egui;
use egui::{Context, Ui};
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
    show_archived: bool,
    show_unknown_mods: bool,
    should_scroll_into_view: Option<ProjectLnk>,
    expanded_depended_on: Option<ProjectLnk>,
}

impl MainPanel {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            sort_popup: SortPopup::new(state.clone()),
            rename_input_open: false,
            rename_input: String::new(),
            search_query: String::new(),
            show_archived: false,
            show_unknown_mods: false,
            should_scroll_into_view: None,
            expanded_depended_on: None,
        }
    }

    pub fn show(&mut self, ctx: &Context, _ui: &mut Ui) {
        let (open_list_lnk, found_files) = {
            let s = self.state.read();
            (s.open_list.clone(), s.found_files.clone())
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            let lnk = match open_list_lnk {
                Some(l) => l,
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

            let (list_name, ver, loader, dir, projects_empty) = {
                let list = list_arc.read();
                let rt_config = list
                    .get_resource_type(&content_type)
                    .expect("List without type");
                (
                    list.get_name(),
                    list.get_game_version().clone(),
                    rt_config.loader.clone(),
                    rt_config.download_dir.clone(),
                    list.manual_projects_by_type(content_type).is_empty(),
                )
            };

            ui.horizontal(|ui| {
                if self.rename_input_open {
                    ui.text_edit_singleline(&mut self.rename_input);
                    if ui.button("✔").clicked() {
                        let mut list = list_arc.write();
                        list.set_list_name(self.rename_input.clone());
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
                            "{} List | {} | {}",
                            content_type.display_name(),
                            ver.name,
                            loader.name
                        ))
                        .small()
                        .weak(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("🗑 Delete")).clicked() {
                            self.state.write().open_list = None;
                            self.state.read().list_pool.delete(&lnk);
                        }
                        if ui.add(egui::Button::new("✏ Rename")).clicked() {
                            self.rename_input = list_name.clone();
                            self.rename_input_open = true;
                        }
                        if ui.add(egui::Button::new("👥 Duplicate")).clicked() {
                            self.state.read().list_pool.duplicate(&lnk);
                        }
                        if ui.add(egui::Button::new("📂 Open Folder")).clicked() {
                            self.state.read().open_explorer(dir.clone().into());
                        }

                        if ui.add(egui::Button::new("📤 Export")).clicked()
                            && let Some(path) = Dialogs::save_export_list_file(
                                &list_name,
                                content_type == ResourceType::Mod,
                            )
                        {
                            let ext = path.extension().and_then(|s| s.to_str());
                            if ext == Some("toml") || ext == Some("mmd") {
                                self.state.read().list_pool.export(&lnk, path);
                            } else if content_type == ResourceType::Mod {
                                self.state.read().list_pool.export_legacy(
                                    &lnk,
                                    path,
                                    ver.clone(),
                                    loader.clone(),
                                );
                            }
                        }

                        let sort_id = self.sort_popup.id();
                        let sort_settings = self.sort_popup.settings.read();
                        let sort_btn = ui.button(match sort_settings.order_mode {
                            OrderMode::Ascending => "⬇ Sort",
                            OrderMode::Descending => "⬆ Sort",
                        });
                        drop(sort_settings);

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

                        if ui.button("⚙ List Settings").clicked() {
                            self.state
                                .read()
                                .submit_modal(Box::new(ListSettingsModal::new(
                                    self.state.clone(),
                                    lnk.clone(),
                                )));
                        }
                    });
                }
            });

            ui.add_space(4.0);

            let found_hashes: HashSet<String> = found_files
                .as_ref()
                .map(|f| f.iter().map(|(_, h)| h.clone()).collect())
                .unwrap_or_default();

            ui.horizontal(|ui| {
                if ui
                    .button(format!("➕ Add {}", content_type.display_name()))
                    .clicked()
                {
                    self.state.read().submit_modal(Box::new(SearchModal::new(
                        self.state.clone(),
                        lnk.clone(),
                        content_type,
                        ver.clone(),
                        loader.clone(),
                    )));
                }
                ui.add_space(10.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Search resources...")
                        .desired_width(200.0),
                );
                if !self.search_query.is_empty() && ui.button("❌").clicked() {
                    self.search_query.clear();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let filtered = self.get_filtered_projects(
                        &list_arc.read(),
                        &content_type,
                        &found_hashes,
                        &ver,
                        &loader,
                    );

                    let missing: Vec<ProjectLnk> = filtered
                        .iter()
                        .filter(|p| {
                            let list = list_arc.read();
                            let proj = list.get_project(p).unwrap();
                            let is_downloaded = proj
                                .get_version()
                                .is_some_and(|v| found_hashes.contains(&v.artifact_hash));
                            !list.is_project_archived(&p) && !is_downloaded
                        })
                        .cloned()
                        .collect();

                    if ui
                        .add_enabled(!missing.is_empty(), egui::Button::new("⬇ Download All"))
                        .clicked()
                    {
                        let list = list_arc.read();
                        for p_lnk in missing {
                            if let Some(proj) = list.get_project(&p_lnk) {
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
                                        &p_lnk,
                                        &proj.get_name(),
                                        latest,
                                        &dir,
                                        &content_type,
                                    );
                                }
                            }
                        }
                    }
                });
            });

            ui.separator();

            if projects_empty {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("No items in this list");
                });
            } else {
                let filtered = self.get_filtered_projects(
                    &list_arc.read(),
                    &content_type,
                    &found_hashes,
                    &ver,
                    &loader,
                );
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let (active, archived): (Vec<_>, Vec<_>) = filtered
                        .into_iter()
                        .partition(|p| !list_arc.read().is_project_archived(p));

                    for p_lnk in active {
                        self.render_project_entry(
                            ui,
                            &list_arc,
                            &p_lnk,
                            &content_type,
                            &ver,
                            &loader,
                            &found_files,
                            &dir,
                            false,
                        );
                    }

                    if !archived.is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        if ui
                            .button(format!(
                                "{} Archived ({})",
                                if self.show_archived { "🔽" } else { "▶" },
                                archived.len()
                            ))
                            .clicked()
                        {
                            self.show_archived = !self.show_archived;
                        }
                        if self.show_archived {
                            for p_lnk in archived {
                                self.render_project_entry(
                                    ui,
                                    &list_arc,
                                    &p_lnk,
                                    &content_type,
                                    &ver,
                                    &loader,
                                    &found_files,
                                    &dir,
                                    false,
                                );
                            }
                        }
                    }

                    let search_lower = self.search_query.to_lowercase();
                    let unknown_files = self.get_unknown_files(
                        &list_arc.read(),
                        &content_type,
                        &found_files,
                        &search_lower,
                    );
                    if !unknown_files.is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        if ui
                            .button(format!(
                                "{} Unknown Files ({})",
                                if self.show_unknown_mods {
                                    "🔽"
                                } else {
                                    "▶"
                                },
                                unknown_files.len()
                            ))
                            .clicked()
                        {
                            self.show_unknown_mods = !self.show_unknown_mods;
                        }
                        if self.show_unknown_mods {
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
    }

    #[allow(clippy::too_many_arguments)]
    fn render_project_entry(
        &mut self,
        ui: &mut Ui,
        list_arc: &Arc<RwLock<ProjectList>>,
        p_lnk: &ProjectLnk,
        rt: &ResourceType,
        g_ver: &GameVersion,
        g_ld: &GameLoader,
        found_files: &Option<Vec<(PathBuf, String)>>,
        dir: &String,
        is_dependency: bool,
    ) {
        let (metadata, versions) = {
            let meta = get_project_metadata!(self.state, p_lnk.clone(), *rt);
            let vers =
                get_project_versions!(self.state, p_lnk.clone(), *rt, g_ver.clone(), g_ld.clone());
            (meta, vers)
        };

        if let (Ok(Some(meta)), Ok(Some(vers))) = (&metadata, &versions)
            && !vers.is_empty()
        {
            let latest = vers.first().unwrap();
            let mut list = list_arc.write();
            let project = list.get_project_mut(p_lnk).unwrap();
            project.update_cache(meta.clone());
            if project.get_version().is_none() {
                drop(list);
                self.state.read().list_pool.select_version(
                    &list_arc.read().get_lnk(),
                    p_lnk.clone(),
                    latest.version_id.clone(),
                );
            }
        }

        let (
            name,
            author,
            _version_id,
            is_archived,
            is_overruled,
            cur_hash,
            has_dependents,
            depended_on,
        ) = {
            let p = list_arc.read();
            let proj = p.get_project(p_lnk).unwrap();
            (
                proj.get_name(),
                proj.get_author(),
                proj.get_version_id().map(|s| s.to_string()),
                list_arc.read().is_project_archived(&p_lnk),
                proj.is_compatibility_overruled(),
                proj.get_version().map(|v| v.artifact_hash.clone()),
                proj.has_dependents(),
                proj.get_version().map(|v| v.get_depended_ons().to_vec()),
            )
        };

        let filename = Self::get_project_filename(&name, rt);
        let file_on_disk = found_files.as_ref().and_then(|files| {
            files
                .iter()
                .find(|(path, _)| path.file_name().is_some_and(|n| n == filename.as_str()))
        });
        let is_file_present = file_on_disk.is_some();
        let disk_hash = file_on_disk.map(|(_, h)| h.clone());

        let has_failed = metadata.is_err() || versions.is_err();
        let has_loaded_files = found_files.is_some();
        let is_downloaded = disk_hash.is_some() && disk_hash == cur_hash;

        let mut is_updatable = false;
        let mut version_name = "⏳";

        if let Ok(Some(vers)) = &versions
            && !vers.is_empty()
        {
            let latest = vers.first().unwrap();
            version_name = &latest.version_name;
            is_updatable = is_file_present && disk_hash.as_ref() != Some(&latest.artifact_hash);
        }

        if version_name.starts_with("v") {
            version_name = &version_name[1..];
        }

        let compatibility = if let Ok(Some(vers)) = &versions {
            Some(!vers.is_empty())
        } else {
            None
        };

        let dl_status = self
            .state
            .read()
            .download_status
            .get(p_lnk)
            .cloned()
            .unwrap_or((DownloadStatus::Idle, 0.0));
        let should_scroll = self.should_scroll_into_view.as_ref() == Some(p_lnk);

        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .corner_radius(6.0)
            .inner_margin(8.0);

        let response = frame.show(ui, |ui| {
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

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let mut name_rich = egui::RichText::new(&name).strong();
                        if is_archived {
                            name_rich = name_rich.weak();
                        }
                        ui.hyperlink_to(name_rich, get_project_link!(self.state, p_lnk, rt));
                    });

                    if has_failed {
                        if ui
                            .button(
                                egui::RichText::new("⚠ Failed to load")
                                    .color(egui::Color32::YELLOW),
                            )
                            .clicked()
                        {
                            clear_project_metadata!(self.state, p_lnk.clone(), *rt);
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!("v{version_name} by {author}"))
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
                                        dep.dependency_type == ProjectDependencyType::Required
                                    })
                                    .collect();

                                if !required_deps.is_empty() {
                                    let is_expanded = self
                                        .expanded_depended_on
                                        .as_ref()
                                        .is_some_and(|id| id == p_lnk);

                                    let badge_text =
                                        format!("+{} Dependencies", required_deps.len());
                                    let mut badge_color = egui::Color32::from_rgb(100, 150, 200);
                                    if is_expanded {
                                        badge_color = egui::Color32::from_rgb(150, 200, 255);
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(badge_text).color(badge_color),
                                            )
                                            .small(),
                                        )
                                        .clicked()
                                    {
                                        if is_expanded {
                                            self.expanded_depended_on = None;
                                        } else {
                                            self.expanded_depended_on = Some(p_lnk.clone());
                                        }
                                    }
                                    ui.add_space(3.0);
                                }
                            }

                            if is_updatable {
                                ui.colored_label(
                                    egui::Color32::from_rgb(100, 200, 255),
                                    "🔄 Update Available",
                                );
                            }
                            if has_loaded_files
                                && !is_archived
                                && !is_downloaded
                                && matches!(compatibility, Some(true))
                            {
                                ui.colored_label(egui::Color32::GOLD, "📁 Missing");
                            }

                            if is_overruled {
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 165, 0),
                                    "⚠ Incompatible Overruled",
                                );
                                if ui.small_button("🔓 Revoke").clicked() {
                                    list_arc
                                        .write()
                                        .get_project_mut(p_lnk)
                                        .unwrap()
                                        .set_compatibility_overruled(false);
                                }
                            } else if matches!(compatibility, Some(false)) {
                                ui.colored_label(egui::Color32::RED, "❌ Incompatible");
                                if ui.small_button("🔒 Overrule").clicked() {
                                    list_arc
                                        .write()
                                        .get_project_mut(p_lnk)
                                        .unwrap()
                                        .set_compatibility_overruled(true);
                                }
                            }
                        });
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_dependency {
                        if ui
                            .add_enabled(!has_dependents, egui::Button::new("🗑"))
                            .on_hover_text("Remove from list")
                            .clicked()
                        {
                            list_arc.write().remove_project(p_lnk);
                        }

                        let archive_label = if is_archived {
                            "📂 Unarchive"
                        } else {
                            "📁 Archive"
                        };
                        if ui
                            .add_enabled(!has_dependents, egui::Button::new(archive_label))
                            .clicked()
                        {
                            let mut list = list_arc.write();
                            let new_state = !list.is_project_archived(&p_lnk);
                            let p = list.get_project_mut(p_lnk).unwrap();
                            p.set_archived(new_state);

                            let path = PathBuf::from(dir);
                            if new_state {
                                self.state.read().archive_file(path, filename.clone());
                            } else {
                                self.should_scroll_into_view = Some(p_lnk.clone());
                                self.state.read().unarchive_file(path, filename.clone());
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
                                let btn_label = if is_updatable {
                                    "🔄 Update"
                                } else {
                                    "Download"
                                };
                                let can_dl = matches!(compatibility, Some(true)) || is_overruled;
                                let ui_enabled =
                                    can_dl && (!is_downloaded || is_updatable) && has_loaded_files;

                                let latest_version = if let Ok(Some(v_list)) = &versions {
                                    v_list.first()
                                } else {
                                    None
                                };

                                let btn = ui.add_enabled(
                                    ui_enabled && latest_version.is_some(),
                                    egui::Button::new(btn_label),
                                );

                                if btn.clicked()
                                    && let Some(v) = latest_version
                                {
                                    self.trigger_download(p_lnk, &name, v, dir, rt);
                                }

                                if is_downloaded && !is_updatable {
                                    ui.label("✅");
                                }
                            }
                        }
                    }
                });
            });
        });
        if !is_dependency {
            if let Some(ref expanded_id) = self.expanded_depended_on.clone() {
                if expanded_id == p_lnk {
                    if let Some(depended_ons) = depended_on {
                        let required_deps: Vec<_> = depended_ons
                            .iter()
                            .filter(|dep| dep.dependency_type == ProjectDependencyType::Required)
                            .collect();

                        if !required_deps.is_empty() {
                            ui.indent("dep_indent", |ui| {
                                ui.add_space(4.0);
                                for dep in &required_deps {
                                    Self::render_project_entry(
                                        self,
                                        ui,
                                        list_arc,
                                        &dep.project,
                                        rt,
                                        g_ver,
                                        g_ld,
                                        found_files,
                                        dir,
                                        true,
                                    );
                                }
                            });
                        }
                    }
                }
            }

            if should_scroll {
                response.response.scroll_to_me(Some(egui::Align::Center));
                self.should_scroll_into_view = None;
            }
        }

        ui.add_space(4.0);
    }

    fn get_filtered_projects(
        &self,
        list: &ProjectList,
        rt: &ResourceType,
        hashes: &HashSet<String>,
        current_ver: &GameVersion,
        current_loader: &GameLoader,
    ) -> Vec<ProjectLnk> {
        let mut mods = list.manual_projects_by_type(*rt);
        let query = self.search_query.to_lowercase();
        let settings = self.sort_popup.settings.read();

        mods.retain(|p| {
            let matches_query = query.is_empty()
                || p.get_name().to_lowercase().contains(&query)
                || p.get_author().to_lowercase().contains(&query);
            let is_downloaded = p
                .get_version()
                .is_some_and(|v| hashes.contains(&v.artifact_hash));

            match settings.filter_mode {
                FilterMode::All => matches_query,
                FilterMode::MissingOnly => matches_query && !is_downloaded && !list
                    .is_project_archived(&p.get_lnk()),
                FilterMode::CompatibleOnly => {
                    let vers_res = get_project_versions!(
                        self.state,
                        p.get_lnk().clone(),
                        *rt,
                        current_ver.clone(),
                        current_loader.clone()
                    );
                    if let Ok(Some(vers)) = vers_res {
                        matches_query && !vers.is_empty()
                    } else {
                        matches_query
                    }
                }
                FilterMode::IncompatibleOnly => {
                    let vers_res = get_project_versions!(
                        self.state,
                        p.get_lnk().clone(),
                        *rt,
                        current_ver.clone(),
                        current_loader.clone()
                    );
                    if let Ok(Some(vers)) = vers_res {
                        matches_query && vers.is_empty()
                    } else {
                        false
                    }
                }
            }
        });

        mods.sort_by(|a, b| match settings.sort_mode {
            SortMode::Name => a
                .get_name()
                .to_lowercase()
                .cmp(&b.get_name().to_lowercase()),
            SortMode::DateAdded => a.added_at.cmp(&b.added_at),
        });

        if settings.order_mode == OrderMode::Descending {
            mods.reverse();
        }

        mods.iter().map(|p| p.get_lnk().clone()).collect()
    }

    fn get_unknown_files(
        &self,
        list: &ProjectList,
        rt: &ResourceType,
        found: &Option<Vec<(PathBuf, String)>>,
        query: &str,
    ) -> Vec<(PathBuf, String)> {
        let known_filenames: HashSet<String> = list
            .manual_projects_by_type(*rt)
            .iter()
            .map(|p| Self::get_project_filename(&p.get_name(), rt))
            .collect();

        found
            .as_ref()
            .map(|f| {
                f.iter()
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
            })
            .unwrap_or_default()
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
                    self.state.read().delete_artifact(
                        path.parent().unwrap().to_path_buf(),
                        filename.to_string(),
                    );
                }
            });
        });
        ui.separator();
    }

    fn trigger_download(
        &self,
        p_lnk: &ProjectLnk,
        p_name: &str,
        version: &RTProjectVersion,
        dir: &String,
        rt: &ResourceType,
    ) {
        let safe_name: String = Self::get_project_filename(p_name, rt);

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

    fn get_project_filename(p_name: &str, rt: &ResourceType) -> String {
        let safe_name: String = p_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect();
        format!("{}.{}", safe_name, rt.file_extension())
    }
}
