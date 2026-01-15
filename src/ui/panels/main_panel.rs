use crate::app::{AppRuntime, AppState, DownloadStatus, Effect, ListAction};
use crate::domain::{ModEntry, ProjectType};
use crate::ui::{ViewState, dialogs::Dialogs};
use eframe::egui;

pub struct MainPanel;

impl MainPanel {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
        runtime: &mut AppRuntime,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            if state.current_list_id.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("No list selected");
                    ui.label("Select a list from the sidebar or create a new one");
                });
                return;
            }

            if state.initial_loading {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.add(egui::Spinner::new().size(32.0));
                    ui.add_space(10.0);
                    ui.label("Loading...");
                });
                return;
            }

            let can_interact = state.current_list_id.is_some();
            let mut content_type = ProjectType::Mod;

            ui.horizontal(|ui| {
                if let Some(list) = state.get_current_list() {
                    content_type = list.content_type;
                    ui.heading(format!("{} {}", list.content_type.emoji(), &list.name));
                    ui.add_space(1.0);

                    let ver = state.get_effective_version();
                    let loader = state.get_effective_loader();
                    let ver_name = state
                        .minecraft_versions
                        .iter()
                        .find(|v| v.id == ver)
                        .map(|v| v.name.as_str())
                        .unwrap_or(&ver);

                    let loader_name = state
                        .mod_loaders
                        .iter()
                        .find(|l| l.id == loader)
                        .map(|l| l.name.as_str())
                        .unwrap_or(&loader);

                    ui.label(
                        egui::RichText::new(format!(
                            "{} List | {} | {}",
                            content_type.display_name(),
                            ver_name,
                            loader_name
                        ))
                        .small()
                        .weak(),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if view_state.show_rename_input {
                        ui.text_edit_singleline(&mut view_state.rename_list_input);
                        if ui.button("✔").clicked() {
                            if let Some(list) = state.get_current_list_mut() {
                                list.name = view_state.rename_list_input.clone();
                                effects.push(Effect::SaveList { list: list.clone() });
                            }
                            view_state.show_rename_input = false;
                        }
                        if ui.button("❌").clicked() {
                            view_state.show_rename_input = false;
                        }
                    } else {
                        if ui
                            .add_enabled(can_interact, egui::Button::new("🗑 Delete"))
                            .clicked()
                        {
                            effects.extend(state.delete_current_list());
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("✏ Rename"))
                            .clicked()
                        {
                            view_state.show_rename_input = true;
                            if let Some(list) = state.get_current_list() {
                                view_state.rename_list_input = list.name.clone();
                            }
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("👥 Duplicate"))
                            .clicked()
                        {
                            if let Some(list) = state.get_current_list().cloned() {
                                view_state.import_name_input = format!("{} (Copy)", list.name);
                                view_state.pending_import_list = Some(list);
                                view_state.active_action = ListAction::Duplicate;
                                view_state.import_window_open = true;
                            }
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("📂 Open Folder"))
                            .on_hover_text("Open download directory")
                            .clicked()
                        {
                            if let Some(list) = state.get_current_list() {
                                let download_dir = if list.download_dir.is_empty() {
                                    state.get_effective_download_dir()
                                } else {
                                    list.download_dir.clone()
                                };

                                #[cfg(target_os = "windows")]
                                {
                                    let _ = std::process::Command::new("explorer")
                                        .arg(&download_dir)
                                        .spawn();
                                }
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = std::process::Command::new("open")
                                        .arg(&download_dir)
                                        .spawn();
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg(&download_dir)
                                        .spawn();
                                }
                            }
                        }

                        if ui
                            .add_enabled(can_interact, egui::Button::new("📤 Export"))
                            .clicked()
                        {
                            if let Some(list) = state.get_current_list() {
                                if let Some(save_path) = Dialogs::save_export_list_file(&list.name)
                                {
                                    effects.extend(state.export_current_list(save_path));
                                }
                            }
                        }

                        let sort_label = match view_state.current_order_mode {
                            crate::app::OrderMode::Ascending => "⬇ Sort",
                            crate::app::OrderMode::Descending => "⬆ Sort",
                        };
                        let sort_btn = ui
                            .add_enabled(can_interact, egui::Button::new(sort_label))
                            .on_hover_text("Sort and Filter");
                        if sort_btn.clicked() {
                            view_state.sort_menu_open = !view_state.sort_menu_open;
                        }
                        view_state.sort_btn_rect = sort_btn.rect;

                        if ui.button("⚙ List Settings").clicked() {
                            view_state.list_settings_open = true;
                            view_state.list_settings_version.clear();
                        }
                    }
                });
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        can_interact,
                        egui::Button::new(format!("➕ Add {}", content_type.display_name())),
                    )
                    .clicked()
                {
                    view_state.search_window_open = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let filtered_mods = state.get_filtered_mods(
                        &view_state.search_query,
                        view_state.current_sort_mode,
                        view_state.current_order_mode,
                        view_state.current_filter_mode,
                    );

                    let missing_ids = state.get_missing_mod_ids(&filtered_mods);

                    let mods_to_download: Vec<String> = filtered_mods
                        .iter()
                        .filter(|entry| {
                            !entry.archived
                                && !state.mods_being_loaded.contains(&entry.mod_id)
                                && state
                                    .download_status
                                    .get(&entry.mod_id)
                                    .map(|s| {
                                        matches!(
                                            s,
                                            DownloadStatus::Idle
                                                | DownloadStatus::Complete
                                                | DownloadStatus::Failed
                                        )
                                    })
                                    .unwrap_or(true)
                                && state.is_mod_compatible(&entry.mod_id).unwrap_or(false)
                        })
                        .map(|e| e.mod_id.clone())
                        .collect();

                    let mods_to_download_count = mods_to_download.len();

                    if ui
                        .add_enabled(
                            can_interact && !mods_to_download.is_empty(),
                            egui::Button::new("⬇ Download All"),
                        )
                        .clicked()
                    {
                        for mod_id in mods_to_download {
                            effects.extend(state.start_download(&mod_id));
                        }
                    }

                    if !missing_ids.is_empty() && missing_ids.len() < mods_to_download_count {
                        ui.add_space(5.0);
                        if ui
                            .add_enabled(
                                can_interact,
                                egui::Button::new(format!(
                                    "⬇ Download Missing ({})",
                                    missing_ids.len()
                                )),
                            )
                            .clicked()
                        {
                            for mod_id in missing_ids {
                                effects.extend(state.start_download(&mod_id));
                            }
                        }
                    }
                });
            });

            ui.separator();

            if let Some(list) = state.get_current_list() {
                if list.mods.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("No items in this list");
                        ui.label("Click 'Add Item' above to get started");
                    });
                } else {
                    let filtered_entries = state.get_filtered_mods(
                        &view_state.search_query,
                        view_state.current_sort_mode,
                        view_state.current_order_mode,
                        view_state.current_filter_mode,
                    );

                    let active_mods: Vec<_> =
                        filtered_entries.iter().filter(|e| !e.archived).collect();
                    let archived_mods: Vec<_> =
                        filtered_entries.iter().filter(|e| e.archived).collect();

                    let unknown_files = state.get_unknown_mod_files();

                    ui.add_space(10.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in &active_mods {
                            Self::render_mod_entry(
                                ui,
                                content_type,
                                entry,
                                state,
                                runtime,
                                &mut effects,
                            );
                        }

                        if !archived_mods.is_empty() {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                let icon = if view_state.show_archived {
                                    "🔽"
                                } else {
                                    "▶"
                                };
                                if ui
                                    .button(format!("{} Archived ({})", icon, archived_mods.len()))
                                    .clicked()
                                {
                                    view_state.show_archived = !view_state.show_archived;
                                }
                            });

                            if view_state.show_archived {
                                ui.add_space(4.0);
                                for entry in &archived_mods {
                                    Self::render_mod_entry(
                                        ui,
                                        content_type,
                                        entry,
                                        state,
                                        runtime,
                                        &mut effects,
                                    );
                                }
                            }
                        }

                        if !unknown_files.is_empty() {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                let icon = if view_state.show_unknown_mods {
                                    "🔽"
                                } else {
                                    "▶"
                                };
                                if ui
                                    .button(format!(
                                        "{} Unknown Files ({})",
                                        icon,
                                        unknown_files.len()
                                    ))
                                    .on_hover_text("Files in download folder without metadata")
                                    .clicked()
                                {
                                    view_state.show_unknown_mods = !view_state.show_unknown_mods;
                                }
                            });

                            if view_state.show_unknown_mods {
                                ui.add_space(4.0);
                                for filename in &unknown_files {
                                    Self::render_unknown_mod_entry(
                                        ui,
                                        filename,
                                        state,
                                        &mut effects,
                                    );
                                }
                            }
                        }
                    });
                }
            }
        });

        if view_state.sort_menu_open {
            Self::show_sort_menu(ctx, view_state);
        }

        effects
    }

    fn render_mod_entry(
        ui: &mut egui::Ui,
        project_type: ProjectType,
        entry: &ModEntry,
        state: &mut AppState,
        runtime: &mut AppRuntime,
        effects: &mut Vec<Effect>,
    ) {
        let mod_id = &entry.mod_id;

        effects.extend(state.load_mod_details_if_needed(mod_id));

        let is_loading = state.mods_being_loaded.contains(mod_id);
        let has_failed = state.mods_failed_loading.contains(mod_id);
        let mod_info = state.get_cached_mod(mod_id);

        let compatibility = state.is_mod_compatible(mod_id);
        let is_missing = !entry.archived && !state.is_mod_downloaded(mod_id);
        let is_updateable = !entry.archived && state.is_mod_updateable(mod_id);

        ui.horizontal(|ui| {
            if let Some(ref info) = mod_info {
                if !info.icon_url.is_empty() {
                    if let Some(handle) = runtime.icon_service.get(&info.icon_url) {
                        ui.add(
                            egui::Image::from_texture(handle)
                                .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                        );
                    } else {
                        ui.add_sized(egui::vec2(32.0, 32.0), egui::Spinner::new());
                    }
                } else {
                    ui.add_space(32.0);
                }
            } else {
                ui.add_sized(egui::vec2(32.0, 32.0), egui::Spinner::new());
            }

            ui.add_space(4.0);

            ui.vertical(|ui| {
                let mut name_text = egui::RichText::new(&entry.mod_name);
                if entry.archived {
                    name_text = name_text.weak();
                }

                let project_link = runtime.get_project_link(&project_type, &entry.mod_id);
                ui.hyperlink_to(name_text, project_link);

                if let Some(ref info) = mod_info {
                    let version_text = if info.version.is_empty() {
                        "Loading version...".to_string()
                    } else {
                        format!("v{}", info.version)
                    };
                    ui.label(format!("{} by {}", version_text, info.author));
                } else if is_loading {
                    ui.label("⏳ Loading details...");
                } else if has_failed {
                    if ui
                        .button(
                            egui::RichText::new("⚠ Failed to load").color(egui::Color32::YELLOW),
                        )
                        .clicked()
                    {
                        effects.extend(state.force_reload_mod(mod_id));
                    }
                }

                let has_override = state.has_compatibility_override(mod_id);
                let raw_compatibility = state.is_mod_compatible_raw(mod_id);

                ui.horizontal(|ui| {
                    if is_updateable {
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 200, 255),
                            "🔄 Update Available",
                        );
                        ui.add_space(3.0);
                    }
                    if is_missing && matches!(compatibility, Some(true)) {
                        ui.colored_label(egui::Color32::YELLOW, "📁 Missing");
                        ui.add_space(3.0);
                    }
                    if has_override {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 165, 0),
                                "⚠ Incompatible Overruled",
                            );
                            if ui.small_button("🔓 Revoke").clicked() {
                                effects.extend(state.toggle_compatibility_override(mod_id));
                            }
                        });
                    } else if matches!(raw_compatibility, Some(false)) {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, "❌ Incompatible");
                            if ui.small_button("🔒 Overrule").clicked() {
                                effects.extend(state.toggle_compatibility_override(mod_id));
                            }
                        });
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑").clicked() {
                    effects.extend(state.delete_mod(mod_id));
                }

                let archive_text = if entry.archived {
                    "📂 Unarchive"
                } else {
                    "📁"
                };
                if ui.button(archive_text).clicked() {
                    effects.extend(state.toggle_archive_mod(mod_id));
                }

                if !entry.archived {
                    let status = state
                        .download_status
                        .get(mod_id)
                        .copied()
                        .unwrap_or(DownloadStatus::Idle);

                    let has_metadata = state.has_download_metadata(mod_id);
                    let is_downloaded = has_metadata && (!is_missing && mod_info.is_some());

                    match status {
                        DownloadStatus::Downloading | DownloadStatus::Queued => {
                            let progress =
                                state.download_progress.get(mod_id).copied().unwrap_or(0.0);
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .text(format!("{:.0}%", progress * 100.0))
                                    .desired_width(80.0),
                            );
                        }
                        any => {
                            let enabled =
                                mod_info.is_some() && !matches!(compatibility, Some(false));
                            let button_text = if is_updateable {
                                "🔄 Update"
                            } else {
                                "Download"
                            };
                            if ui
                                .add_enabled(enabled, egui::Button::new(button_text))
                                .clicked()
                            {
                                effects.extend(state.start_download(mod_id));
                            }
                            if (any == DownloadStatus::Complete || is_downloaded) && !is_updateable
                            {
                                ui.label("✅");
                            }
                            if any == DownloadStatus::Failed {
                                ui.colored_label(egui::Color32::RED, "❌");
                            }
                        }
                    }
                }
            });
        });

        ui.separator();
    }

    fn render_unknown_mod_entry(
        ui: &mut egui::Ui,
        filename: &str,
        state: &AppState,
        effects: &mut Vec<Effect>,
    ) {
        ui.horizontal(|ui| {
            let (rect, _response) =
                ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());

            let text_pos = rect.center() + egui::vec2(4.0, -4.0);

            ui.painter().text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                "❓",
                egui::FontId::proportional(24.0),
                ui.style().visuals.text_color(),
            );

            ui.add_space(4.0);

            ui.vertical(|ui| {
                ui.label(egui::RichText::new(filename).weak());
                ui.label(egui::RichText::new("No metadata available").weak().small());
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑").clicked() {
                    effects.push(Effect::DeleteUnknownFile {
                        download_dir: state.get_effective_download_dir(),
                        filename: filename.to_string(),
                    });
                }
            });
        });

        ui.separator();
    }

    fn show_sort_menu(ctx: &egui::Context, view_state: &mut ViewState) {
        let popup_pos = view_state.sort_btn_rect.left_bottom() + egui::vec2(0.0, 5.0);

        let stored_size = view_state.sort_popup_rect.size();
        let popup_size = if stored_size.x >= 140.0 {
            stored_size
        } else {
            egui::vec2(140.0, 200.0)
        };
        let popup_rect = egui::Rect::from_min_size(popup_pos, popup_size);

        /*//Debug renderer for mouse regions
        {
            use egui::{Color32, Rounding, Stroke};
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("sort_menu_debug"),
            ));
            painter.rect_filled(
                popup_rect,
                Rounding::same(6.0),
                Color32::from_rgba_unmultiplied(50, 120, 200, 30),
            );
            painter.rect_stroke(
                popup_rect,
                Rounding::same(6.0),
                Stroke::new(2.0, Color32::from_rgb(50, 120, 200)),
            );
            painter.rect_stroke(
                view_state.sort_btn_rect,
                Rounding::same(4.0),
                Stroke::new(2.0, Color32::from_rgb(200, 120, 50)),
            );
        }*/

        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !view_state.sort_btn_rect.contains(pos) && !popup_rect.contains(pos) {
                    view_state.sort_menu_open = false;
                    return;
                }
            }
        }

        egui::Area::new(egui::Id::new("sort_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ctx, |ui| {
                let frame_response = egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(140.0);
                    ui.label("Sort by:");
                    if ui
                        .selectable_value(
                            &mut view_state.current_sort_mode,
                            crate::app::SortMode::Name,
                            "Name",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }
                    if ui
                        .selectable_value(
                            &mut view_state.current_sort_mode,
                            crate::app::SortMode::DateAdded,
                            "Date Added",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }

                    ui.separator();
                    ui.label("Order:");
                    if ui
                        .selectable_value(
                            &mut view_state.current_order_mode,
                            crate::app::OrderMode::Ascending,
                            "⬇ Ascending",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }
                    if ui
                        .selectable_value(
                            &mut view_state.current_order_mode,
                            crate::app::OrderMode::Descending,
                            "⬆ Descending",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }

                    ui.separator();
                    ui.label("Filter:");
                    if ui
                        .selectable_value(
                            &mut view_state.current_filter_mode,
                            crate::app::FilterMode::All,
                            "⭕ All",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }
                    if ui
                        .selectable_value(
                            &mut view_state.current_filter_mode,
                            crate::app::FilterMode::CompatibleOnly,
                            "✅ Compatible",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }
                    if ui
                        .selectable_value(
                            &mut view_state.current_filter_mode,
                            crate::app::FilterMode::IncompatibleOnly,
                            "❎ Incompatible",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }
                    if ui
                        .selectable_value(
                            &mut view_state.current_filter_mode,
                            crate::app::FilterMode::MissingOnly,
                            "❔ Missing",
                        )
                        .clicked()
                    {
                        view_state.sort_menu_open = false;
                    }
                });

                view_state.sort_popup_rect = frame_response.response.rect;
            });
    }
}
