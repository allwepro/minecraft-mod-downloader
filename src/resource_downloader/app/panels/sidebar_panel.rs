use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::components::import_options_component::ImportOptionsComponent;
use crate::resource_downloader::app::modals::create_folder_modal::CreateFolderModal;
use crate::resource_downloader::app::modals::create_modal::CreateModal;
use crate::resource_downloader::app::popups::create_menu_popup::CreateMenuPopup;
use crate::resource_downloader::app::popups::folder_context_menu::FolderContextMenu;
use crate::resource_downloader::app::popups::import_popup::ImportPopup;
use crate::resource_downloader::app::popups::list_context_menu::ListContextMenu;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::folder_actions::FolderActions;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::domain::{FolderLnk, ListLnk, ResourceType};
use eframe::egui;
use egui::{Color32, StrokeKind, Ui};
use std::collections::{HashMap, HashSet};

pub struct SidebarPanel {
    state: SharedRDState,
    list_search_query: String,
    #[allow(dead_code)]
    new_list_modal: CreateModal,
    #[allow(dead_code)]
    new_folder_modal: CreateFolderModal,
    create_menu_popup: CreateMenuPopup,
    import_popup: ImportPopup,
    context_menu_target: Option<(ListLnk, egui::Rect)>,
    folder_context_menu_target: Option<(FolderLnk, String, egui::Rect)>,
    import_options: ImportOptionsComponent,
    #[allow(dead_code)]
    drag_state: Option<DragState>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum DragState {
    List(ListLnk),
    Folder(FolderLnk),
}

impl SidebarPanel {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_search_query: String::new(),
            new_list_modal: CreateModal::new(state.clone()),
            new_folder_modal: CreateFolderModal::new(state.clone()),
            create_menu_popup: CreateMenuPopup::new(state.clone()),
            import_popup: ImportPopup::new(state.clone()),
            context_menu_target: None,
            folder_context_menu_target: None,
            import_options: ImportOptionsComponent::new(state.clone()),
            drag_state: None,
        }
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.list_search_query)
                .hint_text("🔍 Search Lists...")
                .desired_width(ui.available_width()),
        );

        let offline_mode = self.state.read().offline_mode;

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let button_width = ui.available_width() - 35.0;
            let mut create_btn =
                egui::Button::new(egui::RichText::new("➕ Create").color(if offline_mode {
                    Color32::GRAY
                } else {
                    Color32::LIGHT_GREEN
                }));

            if offline_mode {
                create_btn = create_btn.fill(Color32::from_rgba_unmultiplied(100, 100, 100, 50));
            }

            let res = ui
                .add_enabled_ui(!offline_mode, |ui| {
                    ui.add_sized([button_width, 25.0], create_btn)
                        .on_disabled_hover_text("Disabled in offline mode")
                })
                .inner;

            if res.clicked() {
                self.state
                    .read()
                    .popup_manager
                    .toggle(self.create_menu_popup.id());
            }
            self.state
                .read()
                .popup_manager
                .register_interaction_area(self.create_menu_popup.id(), res.rect);

            self.state
                .read()
                .popup_manager
                .request_show(Box::new(self.create_menu_popup.clone()), res.rect);

            let import_btn = ui
                .add_enabled_ui(!offline_mode, |ui| {
                    ui.add_sized([25.0, 25.0], egui::Button::new("📥"))
                        .on_disabled_hover_text("Disabled in offline mode")
                        .on_hover_text("Import")
                })
                .inner;

            if import_btn.clicked() {
                self.state
                    .read()
                    .popup_manager
                    .toggle(self.import_popup.id());
            }
            self.state
                .read()
                .popup_manager
                .register_interaction_area(self.import_popup.id(), import_btn.rect);

            self.state
                .read()
                .popup_manager
                .request_show(Box::new(self.import_popup.clone()), import_btn.rect);
        });

        ui.add_space(5.0);

        let (open_list, pending_list_scroll) = {
            let mut state = self.state.write();
            (state.open_list.clone(), state.pending_list_scroll.take())
        };

        let list_items: Vec<(ListLnk, ResourceType, String, String, String, String, usize)> = {
            let state = self.state.read();
            state.list_pool.map_filter(|list| {
                let resource_type = list
                    .get_resource_types()
                    .first()
                    .cloned()
                    .unwrap_or(ResourceType::Mod);
                let type_icon = resource_type.emoji();
                let loader_display = list
                    .get_resource_type_config(&resource_type)
                    .map(|c| c.get_loader().name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                let version_display = list.get_game_version().name.clone();

                Some((
                    list.get_lnk(),
                    resource_type,
                    type_icon,
                    list.get_name(),
                    version_display,
                    loader_display,
                    list.count_manual_projects_by_type(resource_type),
                ))
            })
        };

        let total_lists = list_items.len();
        let is_searching = !self.list_search_query.is_empty();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());

            if total_lists == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label(egui::RichText::new("No lists yet!").weak());
                    ui.label(egui::RichText::new("You can easily import one:").weak());
                    ui.add_space(10.0);
                    self.import_options.render_contents(ui);
                });
                return;
            }

            if ui.input(|i| i.pointer.is_decidedly_dragging()) {
                let clip_rect = ui.clip_rect();
                if let Some(pointer_pos) = ui.ctx().pointer_hover_pos()
                    && clip_rect.contains(pointer_pos)
                {
                    let margin = 20.0;
                    let speed = 5.0;
                    if pointer_pos.y < clip_rect.min.y + margin {
                        ui.scroll_with_delta(egui::vec2(0.0, speed));
                        ui.ctx().request_repaint();
                    } else if pointer_pos.y > clip_rect.max.y - margin {
                        ui.scroll_with_delta(egui::vec2(0.0, -speed));
                        ui.ctx().request_repaint();
                    }
                }
            }

            if is_searching {
                // Search mode: flat list view
                let query = self.list_search_query.to_lowercase();
                let filtered_items: Vec<_> = list_items
                    .into_iter()
                    .filter(|item| item.3.to_lowercase().contains(&query))
                    .collect();

                if filtered_items.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("No matching lists").weak());
                    });
                    return;
                }

                for item in filtered_items {
                    let response = self.render_row(ui, &item, &open_list, &pending_list_scroll);
                    let response = ui.interact(response.rect, response.id, egui::Sense::click());

                    if response.hovered() {
                        ui.painter().rect_stroke(
                            response.rect,
                            4.0,
                            egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),
                            StrokeKind::Middle,
                        );
                    }
                    if response.clicked() {
                        self.handle_list_click(&item.0, &open_list);
                    }
                    if let Some((target_lnk, _)) = &self.context_menu_target
                        && target_lnk == &item.0
                    {
                        self.context_menu_target = Some((item.0.clone(), response.rect));
                    }
                    if response.secondary_clicked() {
                        self.context_menu_target = Some((item.0.clone(), response.rect));
                        let menu_id = egui::Id::new("list_context_menu").with(&item.0);
                        self.state.read().popup_manager.toggle(menu_id);
                    }
                }
            } else {
                self.render_folder_structure(ui, list_items, &open_list, &pending_list_scroll);
            }
        });

        if let Some((target_lnk, rect)) = &self.context_menu_target {
            let menu = ListContextMenu::new(self.state.clone(), target_lnk.clone());
            let menu_id = menu.id();
            let pm = self.state.read().popup_manager.clone();
            if pm.is_open(menu_id) {
                pm.register_interaction_area(menu_id, *rect);
                pm.request_show(Box::new(menu), *rect);
            } else {
                self.context_menu_target = None;
            }
        }

        if let Some((folder_lnk, folder_name, rect)) = &self.folder_context_menu_target {
            let menu =
                FolderContextMenu::new(self.state.clone(), folder_lnk.clone(), folder_name.clone());
            let menu_id = menu.id();
            let pm = self.state.read().popup_manager.clone();
            if pm.is_open(menu_id) {
                pm.register_interaction_area(menu_id, *rect);
                pm.request_show(Box::new(menu), *rect);
            } else {
                self.folder_context_menu_target = None;
            }
        }
    }

    fn render_folder_structure(
        &mut self,
        ui: &mut Ui,
        list_items: Vec<(ListLnk, ResourceType, String, String, String, String, usize)>,
        open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
    ) {
        let (folders, folder_assignments, mut folder_order) = {
            let state = self.state.read();
            let config = state.config.read();
            (
                config.folders.clone(),
                config.folder_assignments.clone(),
                config.folder_order.clone(),
            )
        };

        let mut lists_by_folder: HashMap<Option<String>, Vec<_>> = HashMap::new();
        for item in list_items {
            let folder_id = folder_assignments.get(&item.0.to_string()).cloned();
            lists_by_folder.entry(folder_id).or_default().push(item);
        }

        for (folder_id, lists) in lists_by_folder.iter_mut() {
            if folder_id.is_none() {
                // Root level lists use global order
                let (changed, order) = {
                    let state = self.state.read();
                    let config = state.config.read();
                    let mut order = config.list_order.clone();
                    let available: HashSet<String> =
                        lists.iter().map(|i| i.0.to_string()).collect();
                    let mut changed = false;

                    let initial_len = order.len();
                    order.retain(|id| available.contains(id));
                    if order.len() != initial_len {
                        changed = true;
                    }

                    let current_order_set: HashSet<String> = order.iter().cloned().collect();
                    let mut new_items: Vec<_> = lists
                        .iter()
                        .filter(|i| !current_order_set.contains(&i.0.to_string()))
                        .collect();

                    if !new_items.is_empty() {
                        changed = true;
                        new_items.sort_by(|a, b| a.3.to_lowercase().cmp(&b.3.to_lowercase()));
                        for item in new_items {
                            order.push(item.0.to_string());
                        }
                    }
                    (changed, order)
                };

                if changed {
                    ListActions::set_list_order(self.state.clone(), order.clone());
                }

                let mut item_map: HashMap<String, _> =
                    lists.drain(..).map(|i| (i.0.to_string(), i)).collect();
                *lists = order
                    .into_iter()
                    .filter_map(|id| item_map.remove(&id))
                    .collect();
            } else {
                lists.sort_by(|a, b| a.3.to_lowercase().cmp(&b.3.to_lowercase()));
            }
        }

        // Clean up folder order
        {
            let available_folders: HashSet<String> = folders.iter().map(|f| f.id.clone()).collect();
            let initial_len = folder_order.len();
            folder_order.retain(|id| available_folders.contains(id));

            if folder_order.len() != initial_len {
                let mut missing: Vec<_> = available_folders
                    .iter()
                    .filter(|id| !folder_order.contains(id))
                    .cloned()
                    .collect();
                missing.sort();
                folder_order.extend(missing);
                FolderActions::set_folder_order(self.state.clone(), folder_order.clone());
            }
        }

        let mut folder_map: HashMap<String, _> =
            folders.into_iter().map(|f| (f.id.clone(), f)).collect();

        for folder_id in folder_order.clone() {
            if let Some(folder) = folder_map.get(&folder_id) {
                // Only render root-level folders (no parent)
                if folder.parent_id.is_none() {
                    if let Some(folder) = folder_map.remove(&folder_id) {
                        self.render_folder_recursive(
                            ui,
                            folder,
                            &mut folder_map,
                            &lists_by_folder,
                            open_list,
                            pending_list_scroll,
                            0, // depth level
                        );
                    }
                }
            }
        }

        // Render root-level lists
        if let Some(root_lists) = lists_by_folder.get(&None) {
            for item in root_lists {
                self.render_list_with_dnd(ui, item, open_list, pending_list_scroll);
            }
        }

        if ui.input(|i| i.pointer.is_decidedly_dragging()) {
            ui.add_space(8.0);

            let drop_zone_height = 40.0;
            let drop_zone = ui.allocate_response(
                egui::vec2(ui.available_width(), drop_zone_height),
                egui::Sense::hover(),
            );

            if let Some(payload) = drop_zone.dnd_hover_payload::<String>() {
                let (text, color) = if payload.starts_with("folder:") {
                    (
                        "📁 Drop folder to move to root",
                        Color32::from_rgba_unmultiplied(100, 200, 100, 150),
                    )
                } else {
                    (
                        "📋 Drop list to remove from folder",
                        Color32::from_rgba_unmultiplied(150, 150, 150, 150),
                    )
                };

                ui.painter().rect_stroke(
                    drop_zone.rect,
                    4.0,
                    egui::Stroke::new(2.0, color),
                    StrokeKind::Middle,
                );
                ui.painter().rect_filled(
                    drop_zone.rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(80, 80, 80, 30),
                );

                let font_id = egui::FontId::proportional(12.0);
                ui.painter().text(
                    drop_zone.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    font_id,
                    color,
                );
            }

            if let Some(payload) = drop_zone.dnd_release_payload::<String>() {
                if payload.starts_with("folder:") {
                    let folder_id = payload.strip_prefix("folder:").unwrap().to_string();
                    FolderActions::move_folder_to_parent(self.state.clone(), folder_id, None);
                } else {
                    FolderActions::move_list_to_folder(
                        self.state.clone(),
                        (*payload).clone(),
                        None,
                    );
                }
            }
        }
    }

    fn render_folder(
        &mut self,
        ui: &mut Ui,
        folder: crate::resource_downloader::domain::Folder,
        lists: Vec<(ListLnk, ResourceType, String, String, String, String, usize)>,
        open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
    ) {
        let folder_lnk = FolderLnk::new(folder.id.clone());
        let list_count = lists.len();

        let is_folder_selected = {
            let state = self.state.read();
            state.open_folder.as_ref() == Some(&folder_lnk)
        };

        let is_list_in_folder_selected = if let Some(selected_list) = open_list {
            lists.iter().any(|(list_lnk, ..)| list_lnk == selected_list)
        } else {
            false
        };

        ui.add_space(2.0);

        let mut folder_frame = egui::Frame::default()
            .inner_margin(egui::Margin {
                left: 4,
                right: 8,
                top: 4,
                bottom: 4,
            })
            .corner_radius(4.0);

        if is_folder_selected {
            folder_frame = folder_frame
                .fill(ui.visuals().faint_bg_color)
                .stroke(egui::Stroke::new(1.0, Color32::from_gray(100)));
        } else if is_list_in_folder_selected {
            folder_frame = folder_frame
                .fill(Color32::from_rgba_unmultiplied(100, 150, 200, 30)) // Blue background
                .stroke(egui::Stroke::new(
                    2.0,
                    Color32::from_rgba_unmultiplied(100, 150, 200, 150),
                )); // Blue border
        }

        let is_drag_target = ui.input(|i| i.pointer.is_decidedly_dragging());

        let folder_id_payload = format!("folder:{}", folder.id);
        let dnd_id = ui.make_persistent_id("folder_dnd").with(&folder.id);

        let folder_response = folder_frame.show(ui, |ui| {
            let header = egui::CollapsingHeader::new("")
                .id_salt(&folder.id)
                .default_open(!folder.collapsed)
                .show_background(false);

            let header_response = header.show(ui, |ui| {
                ui.indent(&folder.id, |ui| {
                    for item in &lists {
                        self.render_list_with_dnd(ui, item, open_list, pending_list_scroll);
                    }
                });
            });

            let header_rect = header_response.header_response.rect;

            ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);

                    let icon = if !folder.collapsed { "📂" } else { "📁" };

                    let drag_handle = ui.dnd_drag_source(dnd_id, folder_id_payload.clone(), |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new("≡")
                                    .size(14.0)
                                    .color(Color32::from_gray(120)),
                            )
                            .sense(egui::Sense::hover()),
                        )
                        .on_hover_cursor(egui::CursorIcon::Grab)
                        .on_hover_text("Drag to move folder")
                    });

                    ui.add_space(4.0);

                    let label_response = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{} {}", icon, folder.name)).strong(),
                        )
                        .sense(egui::Sense::click()),
                    );

                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(100, 150, 200, 50))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(100, 150, 200)))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(list_count.to_string())
                                    .small()
                                    .color(Color32::from_rgb(200, 220, 255)),
                            );
                        });

                    if label_response.clicked() {
                        self.state.write().set_open_folder(Some(folder_lnk.clone()));
                    }

                    if label_response.secondary_clicked() {
                        self.folder_context_menu_target =
                            Some((folder_lnk.clone(), folder.name.clone(), label_response.rect));
                        let menu_id = egui::Id::new("folder_context_menu").with(&folder.id);
                        self.state.read().popup_manager.toggle(menu_id);
                    }
                });
            });

            if header_response.header_response.clicked() {
                FolderActions::toggle_folder_collapsed(self.state.clone(), folder_lnk.clone());
            }

            header_response
        });

        let outer_response = folder_response.response;

        // Handle folder drop target with enhanced visual feedback
        if is_drag_target {
            let folder_rect = outer_response.rect;

            if let Some(hover_pos) = ui.ctx().pointer_hover_pos() {
                if folder_rect.contains(hover_pos) {
                    if let Some(payload) =
                        ui.memory(|mem| mem.data.get_temp::<String>(egui::Id::new("dnd_payload")))
                    {
                        let (stroke_color, fill_color, text) = if payload.starts_with("folder:") {
                            (
                                Color32::from_rgba_unmultiplied(100, 200, 100, 200),
                                Color32::from_rgba_unmultiplied(100, 200, 100, 40),
                                "📁 Move folder here",
                            )
                        } else {
                            (
                                Color32::from_rgba_unmultiplied(100, 150, 255, 200),
                                Color32::from_rgba_unmultiplied(100, 150, 255, 40),
                                "📋 Move list to folder",
                            )
                        };

                        ui.painter().rect_stroke(
                            folder_rect,
                            4.0,
                            egui::Stroke::new(3.0, stroke_color),
                            StrokeKind::Middle,
                        );
                        ui.painter().rect_filled(folder_rect, 4.0, fill_color);

                        ui.painter().text(
                            folder_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(10.0),
                            stroke_color,
                        );
                    }
                }
            }

            let folder_sense = ui.interact(
                outer_response.rect,
                egui::Id::new("folder_drop").with(&folder.id),
                egui::Sense::hover(),
            );

            if let Some(payload) = folder_sense.dnd_release_payload::<String>() {
                if payload.starts_with("folder:") {
                    let dragged_folder_id = payload.strip_prefix("folder:").unwrap().to_string();
                    if dragged_folder_id != folder.id {
                        // Don't allow dropping on itself
                        FolderActions::move_folder_to_parent(
                            self.state.clone(),
                            dragged_folder_id,
                            Some(folder.id.clone()),
                        );
                    }
                } else {
                    FolderActions::move_list_to_folder(
                        self.state.clone(),
                        (*payload).clone(),
                        Some(folder_lnk.clone()),
                    );
                }
            }
        }

        ui.add_space(2.0);

        ui.add_space(2.0);
    }

    fn render_folder_recursive(
        &mut self,
        ui: &mut Ui,
        folder: crate::resource_downloader::domain::Folder,
        folder_map: &mut HashMap<String, crate::resource_downloader::domain::Folder>,
        lists_by_folder: &HashMap<
            Option<String>,
            Vec<(ListLnk, ResourceType, String, String, String, String, usize)>,
        >,
        open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
        depth: usize,
    ) {
        let folder_lnk = FolderLnk::new(folder.id.clone());

        let lists = lists_by_folder
            .get(&Some(folder_lnk.id().to_string()))
            .cloned()
            .unwrap_or_default();

        self.render_folder(ui, folder.clone(), lists, open_list, pending_list_scroll);

        if !folder.collapsed {
            let subfolders: Vec<String> = folder_map
                .values()
                .filter(|f| f.parent_id.as_ref() == Some(&folder.id))
                .map(|f| f.id.clone())
                .collect();

            for subfolder_id in subfolders {
                if let Some(subfolder) = folder_map.remove(&subfolder_id) {
                    ui.indent(format!("folder_indent_{}", depth), |ui| {
                        self.render_folder_recursive(
                            ui,
                            subfolder,
                            folder_map,
                            lists_by_folder,
                            open_list,
                            pending_list_scroll,
                            depth + 1,
                        );
                    });
                }
            }
        }
    }

    fn render_list_with_dnd(
        &mut self,
        ui: &mut Ui,
        item: &(ListLnk, ResourceType, String, String, String, String, usize),
        open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
    ) {
        let item_id = ui.make_persistent_id("list_dnd").with(&item.0);
        let payload = item.0.to_string();

        let inner_response = ui.dnd_drag_source(item_id, payload, |ui| {
            self.render_row(ui, item, open_list, pending_list_scroll)
        });

        let drag_response = inner_response
            .response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if drag_response.hovered() {
            ui.painter().rect_stroke(
                drag_response.rect,
                4.0,
                egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),
                StrokeKind::Middle,
            );
        }

        if drag_response.clicked() {
            self.handle_list_click(&item.0, open_list);
        }

        if let Some((target_lnk, _)) = &self.context_menu_target
            && target_lnk == &item.0
        {
            self.context_menu_target = Some((item.0.clone(), drag_response.rect));
        }

        if drag_response.secondary_clicked() {
            self.context_menu_target = Some((item.0.clone(), drag_response.rect));
            let menu_id = egui::Id::new("list_context_menu").with(&item.0);
            self.state.read().popup_manager.toggle(menu_id);
        }
    }

    fn handle_list_click(&self, list: &ListLnk, _open_list: &Option<ListLnk>) {
        ListActions::toggle_open_list(self.state.clone(), list);
    }

    fn render_row(
        &self,
        ui: &mut Ui,
        item: &(ListLnk, ResourceType, String, String, String, String, usize),
        open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
    ) -> egui::Response {
        let (list, resource_type, icon, name, version, loader, count) = item;
        let is_selected = open_list.clone().is_some_and(|l| l == *list);
        let should_scroll = pending_list_scroll.as_ref().is_some_and(|l| l == list);

        let padding = egui::Margin {
            left: 8,
            right: 8,
            top: 6,
            bottom: 6,
        };
        let mut frame = egui::Frame::default()
            .inner_margin(padding)
            .corner_radius(4);

        if is_selected {
            frame = frame
                .fill(ui.visuals().faint_bg_color)
                .stroke(egui::Stroke::new(1.0, Color32::from_gray(100)));
        }

        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(icon);
                        ui.add(egui::Label::new(egui::RichText::new(name).strong()).truncate());
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let badge_bg = ui.visuals().widgets.noninteractive.bg_fill;
                        let badge_fill = Color32::from_rgb(
                            badge_bg.r().saturating_add(15),
                            badge_bg.g().saturating_add(15),
                            badge_bg.b().saturating_add(15),
                        );

                        egui::Frame::default()
                            .fill(badge_fill)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(100, 100, 0)))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(resource_type.display_name()).small());
                            });

                        egui::Frame::default()
                            .fill(badge_fill)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0, 100, 0)))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(version).small());
                            });

                        egui::Frame::default()
                            .fill(badge_fill)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0, 50, 150)))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(loader).small());
                            });

                        egui::Frame::default()
                            .fill(badge_fill)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(25, 25, 25)))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(format!("{count}")).small());
                            });
                    });
                });
            })
            .response;

        if should_scroll {
            response.scroll_to_me(Some(egui::Align::Center));
        }

        response
    }
}
