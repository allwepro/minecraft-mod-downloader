use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::components::import_options_component::ImportOptionsComponent;
use crate::resource_downloader::app::modals::create_list_group_modal::CreateListGroupModal;
use crate::resource_downloader::app::modals::create_modal::CreateModal;
use crate::resource_downloader::app::popups::create_menu_popup::CreateMenuPopup;
use crate::resource_downloader::app::popups::import_popup::ImportPopup;
use crate::resource_downloader::app::popups::list_context_menu::ListContextMenu;
use crate::resource_downloader::app::popups::list_group_context_menu::ListGroupContextMenu;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::domain::{
    ListGroup, ListGroupLnk, ListLnk, ResourceType, SidebarItem,
};
use eframe::egui;
use egui::{Color32, StrokeKind, Ui};
use std::collections::HashMap;

type ListItem = (ListLnk, ResourceType, String, String, String, String, usize);

struct RenderContext<'a> {
    open_list: &'a Option<ListLnk>,
    pending_sidebar_scroll: &'a Option<SidebarItem>,
}

impl<'a> RenderContext<'a> {
    fn new(
        open_list: &'a Option<ListLnk>,
        pending_sidebar_scroll: &'a Option<SidebarItem>,
    ) -> Self {
        Self {
            open_list,
            pending_sidebar_scroll,
        }
    }
}

#[derive(Clone)]
struct DropTarget {
    dnd_id: String,
    rect: egui::Rect,
    parent_id: Option<ListGroupLnk>,
    depth: usize,
}

#[derive(PartialEq)]
enum DropPosition {
    Before,
    After,
}

struct ClosestTarget {
    target: DropTarget,
    position: DropPosition,
}

pub struct SidebarPanel {
    state: SharedRDState,
    list_search_query: String,
    #[allow(dead_code)]
    new_list_modal: CreateModal,
    #[allow(dead_code)]
    new_folder_modal: CreateListGroupModal,
    create_menu_popup: CreateMenuPopup,
    import_popup: ImportPopup,
    context_menu_target: Option<(ListLnk, egui::Rect)>,
    folder_context_menu_target: Option<(ListGroupLnk, String, egui::Rect)>,
    import_options: ImportOptionsComponent,
    drop_targets: Vec<DropTarget>,
}

impl SidebarPanel {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_search_query: String::new(),
            new_list_modal: CreateModal::new(state.clone()),
            new_folder_modal: CreateListGroupModal::new(state.clone()),
            create_menu_popup: CreateMenuPopup::new(state.clone()),
            import_popup: ImportPopup::new(state.clone()),
            context_menu_target: None,
            folder_context_menu_target: None,
            import_options: ImportOptionsComponent::new(state.clone()),
            drop_targets: Vec::new(),
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

        let (open_list, pending_sidebar_scroll) = {
            let mut state = self.state.write();
            (state.open_list.clone(), state.pending_sidebar_scroll.take())
        };

        let list_items: Vec<ListItem> = {
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
                    let response = self.render_row(
                        ui,
                        &item,
                        &open_list,
                        &pending_sidebar_scroll
                            .clone()
                            .and_then(|a| a.list_lnk().clone()),
                    );
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
                self.render_sidebar_content(ui, list_items, &open_list, &pending_sidebar_scroll);
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
            let menu = ListGroupContextMenu::new(
                self.state.clone(),
                folder_lnk.clone(),
                folder_name.clone(),
            );
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

    fn render_sidebar_content(
        &mut self,
        ui: &mut Ui,
        list_items: Vec<ListItem>,
        open_list: &Option<ListLnk>,
        pending_sidebar_scroll: &Option<SidebarItem>,
    ) {
        let (folders, list_group_assignments, mut sidebar_order) = {
            let state = self.state.read();
            let config = state.config.read();
            (
                config.list_groups.clone(),
                config.list_group_assignments.clone(),
                config.sidebar_ui_order.clone(),
            )
        };

        // Build lookup maps
        let folder_map: HashMap<ListGroupLnk, ListGroup> =
            folders.iter().map(|f| (f.lnk.clone(), f.clone())).collect();

        let list_map: HashMap<ListLnk, ListItem> = list_items
            .iter()
            .map(|item| (item.0.clone(), item.clone()))
            .collect();

        if sidebar_order.is_empty() {
            let mut new_order = Vec::new();

            for list_group in &folders {
                new_order.push(SidebarItem::from(&list_group.lnk));
            }
            for list in &list_items {
                new_order.push(SidebarItem::from(&list.0));
            }

            sidebar_order = new_order.clone();

            ListActions::set_sidebar_ui_order(self.state.clone(), new_order);
        }

        let mut changed = false;
        for folder in &folders {
            if !sidebar_order.contains(&SidebarItem::from(&folder.lnk)) {
                sidebar_order.push(SidebarItem::from(&folder.lnk));
                changed = true;
            }
        }
        for list in &list_items {
            if !sidebar_order.contains(&SidebarItem::from(&list.0)) {
                sidebar_order.push(SidebarItem::from(&list.0));
                changed = true;
            }
        }
        if changed {
            ListActions::set_sidebar_ui_order(self.state.clone(), sidebar_order.clone());
        }

        let mut items_by_parent: HashMap<Option<ListGroupLnk>, Vec<SidebarItem>> = HashMap::new();

        for list_group in folders {
            items_by_parent
                .entry(list_group.parent_id.clone())
                .or_default()
                .push(SidebarItem::ListGroup(list_group.lnk.clone()));
        }

        for list in list_items {
            let parent = list_group_assignments.get(&list.0).cloned();
            items_by_parent
                .entry(parent)
                .or_default()
                .push(SidebarItem::List(list.0.clone()));
        }

        for items in items_by_parent.values_mut() {
            items.sort_by_key(|item| {
                sidebar_order
                    .iter()
                    .position(|id| id == item)
                    .unwrap_or(usize::MAX)
            });
        }

        let render_ctx = RenderContext::new(open_list, pending_sidebar_scroll);

        self.drop_targets.clear();

        let is_dragging = ui.input(|i| i.pointer.is_decidedly_dragging());
        let pointer_pos = ui.ctx().pointer_hover_pos();

        self.render_mixed_list(
            ui,
            &items_by_parent,
            &folder_map,
            &list_map,
            None,
            &render_ctx,
            0,
        );

        let has_payload = ui.ctx().memory(|mem| {
            mem.data
                .get_temp::<String>(egui::Id::new("dnd_last_payload"))
                .is_some()
        });

        if is_dragging
            && has_payload
            && let Some(pos) = pointer_pos
            && let Some(closest) = self.find_closest_target(pos)
        {
            self.visualize_drop_target(ui, &closest);
        }

        let was_dragging = ui.ctx().memory(|mem| {
            mem.data
                .get_temp::<String>(egui::Id::new("dnd_last_payload"))
                .is_some()
        });

        if was_dragging
            && ui.input(|i| i.pointer.any_released())
            && let Some(pos) = pointer_pos
        {
            let payload_opt = ui.ctx().memory(|mem| {
                mem.data
                    .get_temp::<String>(egui::Id::new("dnd_last_payload"))
            });

            if let Some(payload) = payload_opt {
                if let Some(closest) = self.find_closest_target(pos) {
                    self.apply_drop(&closest, &payload);
                }

                ui.ctx().memory_mut(|mem| {
                    mem.data.remove::<String>(egui::Id::new("dnd_last_payload"));
                });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_mixed_list(
        &mut self,
        ui: &mut Ui,
        items_by_parent: &HashMap<Option<ListGroupLnk>, Vec<SidebarItem>>,
        folder_map: &HashMap<ListGroupLnk, ListGroup>,
        list_map: &HashMap<ListLnk, ListItem>,
        parent_id: Option<ListGroupLnk>,
        ctx: &RenderContext,
        depth: usize,
    ) {
        let empty_vec = Vec::new();
        let items = items_by_parent.get(&parent_id).unwrap_or(&empty_vec);

        for (index, item) in items.iter().enumerate() {
            let is_last_in_group = index == items.len() - 1;

            match item {
                SidebarItem::List(list_lnk) => {
                    if let Some(list_item) = list_map.get(list_lnk) {
                        self.render_list_with_dnd(
                            ui,
                            list_item,
                            ctx.open_list,
                            &ctx.pending_sidebar_scroll
                                .as_ref()
                                .and_then(|s| s.list_lnk()),
                            parent_id.clone(),
                            depth,
                        );
                    }
                }
                SidebarItem::ListGroup(lg_lnk) => {
                    if let Some(list_group) = folder_map.get(lg_lnk) {
                        self.render_list_group_new(
                            ui,
                            list_group.clone(),
                            items_by_parent,
                            folder_map,
                            list_map,
                            ctx,
                            depth,
                            is_last_in_group,
                        );
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_list_group_new(
        &mut self,
        ui: &mut Ui,
        list_group: ListGroup,
        items_by_parent: &HashMap<Option<ListGroupLnk>, Vec<SidebarItem>>,
        folder_map: &HashMap<ListGroupLnk, ListGroup>,
        list_map: &HashMap<ListLnk, ListItem>,
        ctx: &RenderContext,
        depth: usize,
        is_last_in_group: bool,
    ) {
        let lg_lnk = list_group.lnk.clone();
        let id = ui.make_persistent_id("folder").with(&list_group.lnk);
        let payload = format!("folder:{}", list_group.lnk);

        if ui.ctx().is_being_dragged(id) {
            ui.ctx().memory_mut(|mem| {
                mem.data
                    .insert_temp(egui::Id::new("dnd_last_payload"), payload.clone());
            });
        }

        let mut collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            id,
            !list_group.collapsed,
        );

        let mut arrow_clicked = false;

        let folder_response = ui
            .horizontal(|ui| {
                let arrow = ui.add(
                    egui::Button::new(if collapsing.is_open() {
                        "🔽"
                    } else {
                        "▶️"
                    })
                    .frame(false)
                    .small(),
                );
                if arrow.clicked() {
                    arrow_clicked = true;
                }
                let inner_response = ui.dnd_drag_source(id, payload.clone(), |ui| {
                    ui.horizontal(|ui| {
                        let icon = if collapsing.is_open() { "📂" } else { "📁" };

                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("{} {}", icon, list_group.name))
                                    .strong(),
                            )
                            .selectable(false),
                        );
                    })
                    .response
                });

                let drag_response = inner_response
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                if drag_response.clicked() && !arrow_clicked {
                    if let Some(lg) = { self.state.read().open_list_group.clone() }
                        && lg == lg_lnk
                    {
                        self.state.write().set_open_list_group(None);
                    } else {
                        self.state.write().set_open_list_group(Some(lg_lnk.clone()));
                    }
                }

                if drag_response.secondary_clicked() {
                    self.folder_context_menu_target =
                        Some((lg_lnk.clone(), list_group.name.clone(), drag_response.rect));
                    let menu_id = egui::Id::new("list_group_context_menu").with(&lg_lnk);
                    self.state.read().popup_manager.toggle(menu_id);
                }

                if drag_response.double_clicked() {
                    collapsing.toggle(ui);
                    ListGroupActions::toggle_list_group_collapsed(
                        self.state.clone(),
                        lg_lnk.clone(),
                    );
                }

                drag_response
            })
            .inner;

        if let Some(Some(lg_lnk)) = ctx
            .pending_sidebar_scroll
            .clone()
            .map(|s| s.list_group_lnk())
            && lg_lnk == list_group.lnk
        {
            folder_response.scroll_to_me(Some(egui::Align::Center));
        }

        self.drop_targets.push(DropTarget {
            dnd_id: list_group.lnk.to_context_id(),
            rect: folder_response.rect,
            parent_id: list_group.parent_id.clone(),
            depth,
        });

        if arrow_clicked {
            collapsing.toggle(ui);
            ListGroupActions::toggle_list_group_collapsed(self.state.clone(), lg_lnk.clone());
        }

        let is_open = collapsing.is_open();

        collapsing.show_body_unindented(ui, |ui| {
            ui.indent(id, |ui| {
                self.render_mixed_list(
                    ui,
                    items_by_parent,
                    folder_map,
                    list_map,
                    Some(list_group.lnk.clone()),
                    ctx,
                    depth + 1,
                );

                if is_open {
                    let drop_zone_height = 8.0;
                    let drop_zone_response = ui.allocate_response(
                        egui::vec2(ui.available_width(), drop_zone_height),
                        egui::Sense::hover(),
                    );

                    self.drop_targets.push(DropTarget {
                        dnd_id: format!("{}_inside_end", list_group.lnk),
                        rect: drop_zone_response.rect,
                        parent_id: Some(list_group.lnk.clone()),
                        depth: depth + 1,
                    });
                }
            });
        });

        if is_last_in_group {
            let drop_zone_height = 4.0;
            let drop_zone_response = ui.allocate_response(
                egui::vec2(ui.available_width(), drop_zone_height),
                egui::Sense::hover(),
            );

            self.drop_targets.push(DropTarget {
                dnd_id: format!("{}_after", list_group.lnk),
                rect: drop_zone_response.rect,
                parent_id: list_group.parent_id.clone(),
                depth,
            });
        }
    }

    fn parse_payload(&self, payload: &str) -> (String, bool) {
        if let Some(stripped) = payload.strip_prefix("folder:") {
            (stripped.to_string(), true)
        } else {
            (payload.to_string(), false)
        }
    }

    fn move_item(
        &mut self,
        item: SidebarItem,
        new_parent_lg: Option<ListGroupLnk>,
        insert_after: Option<SidebarItem>,
    ) {
        let state = self.state.read();
        let mut config = state.config.read().clone();
        drop(state);

        let is_list_group = config
            .list_groups
            .iter()
            .any(|f| item.match_list_group(&f.lnk));

        if is_list_group {
            if let Some(folder) = config
                .list_groups
                .iter_mut()
                .find(|f| item.match_list_group(&f.lnk))
                && new_parent_lg.as_ref().is_some()
                && item.match_list_group(new_parent_lg.as_ref().unwrap())
            {
                folder.parent_id = new_parent_lg.clone();
            }
        } else if let Some(parent) = new_parent_lg.clone() {
            config
                .list_group_assignments
                .insert(item.list_lnk().unwrap(), parent);
        } else {
            config
                .list_group_assignments
                .remove(&item.list_lnk().unwrap());
        }

        config.sidebar_ui_order.retain(|id| id != &item);

        if let Some(after_id) = insert_after {
            if let Some(pos) = config
                .sidebar_ui_order
                .iter()
                .position(|id| id == &after_id)
            {
                config.sidebar_ui_order.insert(pos + 1, item);
            } else {
                config.sidebar_ui_order.push(item);
            }
        } else {
            config.sidebar_ui_order.insert(0, item);
        }

        let state_guard = self.state.write();
        *state_guard.config.write() = config.clone();
        state_guard.dispatch(crate::resource_downloader::business::Effect::SaveConfig { config });
    }

    fn render_list_with_dnd(
        &mut self,
        ui: &mut Ui,
        item: &ListItem,
        open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
        parent_id: Option<ListGroupLnk>,
        depth: usize,
    ) {
        let item_id = ui.make_persistent_id("list_dnd").with(&item.0);
        let payload = item.0.to_string();

        if ui.ctx().is_being_dragged(item_id) {
            ui.ctx().memory_mut(|mem| {
                mem.data
                    .insert_temp(egui::Id::new("dnd_last_payload"), payload.clone());
            });
        }

        let inner_response = ui.dnd_drag_source(item_id, payload, |ui| {
            self.render_row(ui, item, open_list, pending_list_scroll)
        });

        let drag_response = inner_response
            .response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        self.drop_targets.push(DropTarget {
            dnd_id: item.0.to_string(),
            rect: drag_response.rect,
            parent_id: parent_id.clone(),
            depth,
        });

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
        item: &ListItem,
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

    fn find_closest_target(&self, pointer_pos: egui::Pos2) -> Option<ClosestTarget> {
        let mut closest: Option<ClosestTarget> = None;
        let mut min_distance = f32::MAX;
        const MIN_DISTANCE_THRESHOLD: f32 = 100.0;

        for (index, target) in self.drop_targets.iter().enumerate() {
            let rect = target.rect;
            let is_last = index == self.drop_targets.len() - 1;
            let is_virtual =
                target.dnd_id.ends_with("_after") || target.dnd_id.ends_with("_inside_end");

            let dist_to_top = (pointer_pos.y - rect.min.y).abs();

            if dist_to_top < min_distance {
                min_distance = dist_to_top;
                closest = Some(ClosestTarget {
                    target: target.clone(),
                    position: DropPosition::Before,
                });
            }

            if is_last && !is_virtual {
                let dist_to_bottom = (pointer_pos.y - rect.max.y).abs();

                if dist_to_bottom < min_distance {
                    min_distance = dist_to_bottom;
                    closest = Some(ClosestTarget {
                        target: target.clone(),
                        position: DropPosition::After,
                    });
                }
            }
        }

        if min_distance < MIN_DISTANCE_THRESHOLD {
            closest
        } else {
            None
        }
    }

    fn visualize_drop_target(&self, ui: &mut Ui, closest: &ClosestTarget) {
        let rect = closest.target.rect;
        let indent_step = 15.0;

        match closest.position {
            DropPosition::Before => {
                let y = rect.min.y;
                let x_start = rect.min.x + (closest.target.depth as f32 * indent_step);
                let color = Color32::from_rgb(100, 150, 255);

                ui.painter().line_segment(
                    [egui::pos2(x_start, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(2.0, color),
                );
                ui.painter()
                    .circle_filled(egui::pos2(x_start, y), 3.0, color);
            }
            DropPosition::After => {
                let y = rect.max.y;
                let x_start = rect.min.x + (closest.target.depth as f32 * indent_step);
                let color = Color32::from_rgb(100, 150, 255);

                ui.painter().line_segment(
                    [egui::pos2(x_start, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(2.0, color),
                );
                ui.painter()
                    .circle_filled(egui::pos2(x_start, y), 3.0, color);
            }
        }
    }

    fn apply_drop(&mut self, closest: &ClosestTarget, payload: &str) {
        let (item_id, is_folder_drag) = self.parse_payload(payload);

        let (actual_target_id, is_inside_end) = if closest.target.dnd_id.ends_with("_inside_end") {
            (
                closest
                    .target
                    .dnd_id
                    .trim_end_matches("_inside_end")
                    .to_string(),
                true,
            )
        } else if closest.target.dnd_id.ends_with("_after") {
            (
                closest.target.dnd_id.trim_end_matches("_after").to_string(),
                false,
            )
        } else {
            (closest.target.dnd_id.clone(), false)
        };

        if item_id == actual_target_id {
            return;
        }
        let sidebar_item = {
            self.state
                .read()
                .config
                .read()
                .sidebar_ui_order
                .iter()
                .find(|i| i.to_context_id() == item_id)
                .unwrap()
                .clone()
        };
        let actual_target_item = {
            self.state
                .read()
                .config
                .read()
                .sidebar_ui_order
                .iter()
                .find(|i| i.to_context_id() == actual_target_id)
                .unwrap()
                .clone()
        };

        if is_folder_drag
            && let Some(ref target_parent) = closest.target.parent_id
            && self.is_descendant_of(target_parent, &sidebar_item)
        {
            return;
        }

        if is_inside_end {
            let state = self.state.read();
            let config = state.config.read();

            let current_parent = if is_folder_drag {
                config
                    .list_groups
                    .iter()
                    .find(|f| f.lnk.to_context_id() == item_id)
                    .and_then(|f| f.parent_id.clone())
            } else {
                config
                    .list_group_assignments
                    .get(&ListLnk::new(item_id.clone()))
                    .cloned()
            };

            if current_parent.map(|a| a.to_context_id()) == Some(actual_target_id.clone()) {
                let children_in_order: Vec<SidebarItem> = config
                    .sidebar_ui_order
                    .iter()
                    .filter(|id| {
                        let item_parent = if let Some(folder) = config
                            .list_groups
                            .iter()
                            .find(|f| Some(f.lnk.clone()) == id.list_group_lnk())
                        {
                            folder.parent_id.clone()
                        } else {
                            config
                                .list_group_assignments
                                .get(&id.list_lnk().unwrap())
                                .cloned()
                        };
                        item_parent.map(|a| a.to_context_id()) == Some(actual_target_id.clone())
                    })
                    .cloned()
                    .collect();

                if children_in_order.last().map(|a| a.to_context_id()) == Some(item_id.clone()) {
                    drop(config);
                    drop(state);
                    return;
                }
            }

            let children_in_order: Vec<SidebarItem> = config
                .sidebar_ui_order
                .iter()
                .filter(|id| {
                    let item_parent = if let Some(list_group) = config
                        .list_groups
                        .iter()
                        .find(|f| Some(f.lnk.clone()) == id.list_group_lnk())
                    {
                        list_group.parent_id.clone()
                    } else {
                        config
                            .list_group_assignments
                            .get(&id.list_lnk().unwrap())
                            .cloned()
                    };
                    item_parent.map(|a| a.to_context_id()) == Some(actual_target_id.clone())
                })
                .cloned()
                .collect();

            drop(config);
            drop(state);

            let insert_after = children_in_order.last().cloned();

            self.move_item(
                sidebar_item,
                Some(ListGroupLnk::from(actual_target_id)),
                insert_after,
            );
            return;
        }

        match closest.position {
            DropPosition::Before => {
                if closest.target.dnd_id.ends_with("_after")
                    || closest.target.dnd_id.ends_with("_inside_end")
                {
                    self.move_item(
                        sidebar_item,
                        closest.target.parent_id.clone(),
                        Some(actual_target_item),
                    );
                    return;
                }

                let state = self.state.read();
                let config = state.config.read();

                let siblings_in_order: Vec<SidebarItem> = config
                    .sidebar_ui_order
                    .iter()
                    .filter(|id| {
                        let item_parent = if let Some(folder) = config
                            .list_groups
                            .iter()
                            .find(|f| Some(f.lnk.clone()) == id.list_group_lnk())
                        {
                            folder.parent_id.clone()
                        } else {
                            config
                                .list_group_assignments
                                .get(&id.list_lnk().unwrap())
                                .cloned()
                        };
                        item_parent == closest.target.parent_id
                    })
                    .cloned()
                    .collect();

                drop(config);
                drop(state);

                let target_pos = siblings_in_order
                    .iter()
                    .position(|id| id.to_context_id() == actual_target_id);
                let insert_after = if let Some(pos) = target_pos
                    && pos > 0
                {
                    Some(siblings_in_order[pos - 1].clone())
                } else {
                    None
                };

                let current_pos = siblings_in_order
                    .iter()
                    .position(|id| id.to_context_id() == item_id);
                if let Some(curr_pos) = current_pos
                    && let Some(tgt_pos) = target_pos
                    && (curr_pos == tgt_pos || curr_pos + 1 == tgt_pos)
                {
                    return;
                }

                self.move_item(sidebar_item, closest.target.parent_id.clone(), insert_after);
            }
            DropPosition::After => {
                let state = self.state.read();
                let config = state.config.read();

                let siblings_in_order: Vec<SidebarItem> = config
                    .sidebar_ui_order
                    .iter()
                    .filter(|id| {
                        let item_parent = if let Some(folder) = config
                            .list_groups
                            .iter()
                            .find(|f| Some(f.lnk.clone()) == id.list_group_lnk())
                        {
                            folder.parent_id.clone()
                        } else {
                            config
                                .list_group_assignments
                                .get(&id.list_lnk().unwrap())
                                .cloned()
                        };
                        item_parent == closest.target.parent_id
                    })
                    .cloned()
                    .collect();

                drop(config);
                drop(state);

                let current_pos = siblings_in_order
                    .iter()
                    .position(|id| id.to_context_id() == item_id);
                let target_pos = siblings_in_order
                    .iter()
                    .position(|id| id.to_context_id() == actual_target_id);

                if let Some(curr_pos) = current_pos
                    && let Some(tgt_pos) = target_pos
                    && curr_pos == tgt_pos + 1
                {
                    return;
                }

                self.move_item(
                    sidebar_item,
                    closest.target.parent_id.clone(),
                    Some(actual_target_item),
                );
            }
        }
    }

    fn is_descendant_of(
        &self,
        potential_descendant: &ListGroupLnk,
        potential_ancestor: &SidebarItem,
    ) -> bool {
        if potential_ancestor.match_list_group(potential_descendant) {
            return true;
        }

        let state = self.state.read();
        let config = state.config.read();

        let mut current = potential_descendant;
        while let Some(folder) = config.list_groups.iter().find(|f| f.lnk == *current) {
            if let Some(parent_id) = &folder.parent_id {
                if potential_ancestor.match_list_group(potential_descendant) {
                    return true;
                }
                current = parent_id;
            } else {
                break;
            }
        }

        false
    }
}
