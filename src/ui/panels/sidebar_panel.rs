use crate::app::{AppRuntime, AppState, Effect, ListAction};
use crate::domain::ModList;
use crate::ui::{ViewState, dialogs::Dialogs};
use eframe::egui;

pub struct SidebarPanel;

impl SidebarPanel {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
        _runtime: &mut AppRuntime,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        egui::SidePanel::left("sidebar_panel")
            .resizable(true)
            .default_width(180.0)
            .width_range(150.0..=400.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut view_state.list_search_query)
                        .hint_text("🔍 Search lists...")
                        .desired_width(ui.available_width()),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let button_width = ui.available_width() - 35.0; // Space for one small button
                    if ui
                        .add_sized([button_width, 25.0], egui::Button::new("➕ New List"))
                        .clicked()
                    {
                        view_state.reset_create_list();

                        if let Some(latest_version) = state.minecraft_versions.first() {
                            view_state.new_list_version = latest_version.id.clone();
                        }

                        if let Some(loaders) = state.loaders_for_type(view_state.new_list_type) {
                            if let Some(first_loader) = loaders.first() {
                                view_state.new_list_loader = first_loader.id.clone();
                            }
                        } else {
                            effects.extend(state.ensure_loaders_for_type(view_state.new_list_type));
                        }

                        view_state.create_list_window_open = true;
                    }

                    let import_btn = ui
                        .add_sized([25.0, 25.0], egui::Button::new("📥"))
                        .on_hover_text("Import");

                    let popup_id = ui.make_persistent_id("import_popup");
                    if import_btn.clicked() {
                        view_state.import_popup_open = !view_state.import_popup_open;
                    }

                    if view_state.import_popup_open {
                        let popup_rect = egui::Rect::from_min_size(
                            import_btn.rect.left_bottom(),
                            egui::vec2(150.0, 0.0),
                        );

                        egui::Area::new(popup_id)
                            .order(egui::Order::Foreground)
                            .fixed_pos(popup_rect.min)
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_min_width(150.0);
                                    if ui.button("📄 From File...").clicked() {
                                        view_state.import_popup_open = false;
                                        if let Some(path) = Dialogs::pick_import_list_file() {
                                            match path.extension().and_then(|s| s.to_str()) {
                                                Some("toml") | Some("mmd") => {
                                                    if let Ok(content) =
                                                        std::fs::read_to_string(&path)
                                                        && let Ok(list) =
                                                            toml::from_str::<ModList>(&content)
                                                    {
                                                        view_state.import_name_input =
                                                            format!("{} (Imported)", list.name);
                                                        view_state.pending_import_list = Some(list);
                                                        view_state.active_action =
                                                            ListAction::Import;
                                                        view_state.import_window_open = true;
                                                    }
                                                }
                                                Some("mods") | Some("all-mods")
                                                | Some("queue-mods") => {
                                                    effects.extend(state.start_legacy_import(path));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    if ui.button("🔗 Modrinth Collection...").clicked() {
                                        view_state.import_popup_open = false;
                                        view_state.reset_collection_import();
                                        view_state.collection_import_window_open = true;
                                    }
                                });
                            });

                        if ui.input(|i| i.pointer.any_click()) && !import_btn.clicked() {
                            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
                            if let Some(pos) = pointer_pos
                                && !popup_rect.contains(pos)
                                && !import_btn.rect.contains(pos)
                            {
                                view_state.import_popup_open = false;
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                let list_info: Vec<(String, String, bool)> = state
                    .mod_lists
                    .iter()
                    .filter(|list| {
                        view_state.list_search_query.is_empty()
                            || list
                                .name
                                .to_lowercase()
                                .contains(&view_state.list_search_query.to_lowercase())
                    })
                    .map(|list| {
                        let type_icon = list.content_type.emoji();
                        let display_text = if list.version.is_empty() && list.loader.id.is_empty() {
                            format!("{} {} ({})", type_icon, list.name, list.mods.len())
                        } else {
                            format!(
                                "{} {} [{} | {}] ({})",
                                type_icon,
                                list.name,
                                list.version,
                                if list.loader.name.is_empty() {
                                    &list.loader.id
                                } else {
                                    &list.loader.name
                                },
                                list.mods.len()
                            )
                        };
                        (
                            list.id.clone(),
                            display_text,
                            state.current_list_id.as_ref() == Some(&list.id),
                        )
                    })
                    .collect();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (list_id, display_text, selected) in list_info {
                        if ui.selectable_label(selected, display_text).clicked() {
                            if selected {
                                state.current_list_id = None;
                            } else {
                                state.current_list_id = Some(list_id);
                                effects.extend(state.invalidate_and_reload());

                                let download_dir = state.get_effective_download_dir();
                                effects.push(Effect::ValidateMetadata { download_dir });
                            }
                            view_state.selected_mod = None;
                        }
                    }
                });
            });

        effects
    }
}
