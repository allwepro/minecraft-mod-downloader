use crate::common::ui::ash_ui::AshUi;
use crate::common::ui::structs::popup_window::Popup;
use crate::resource_downloader::app::components::import_options_component::ImportOptionsComponent;
use crate::resource_downloader::app::context_menus::sidebar_panel::list_context_menu::ListContextMenu;
use crate::resource_downloader::app::context_menus::sidebar_panel::list_group_context_menu::ListGroupContextMenu;
use crate::resource_downloader::app::popups::create_menu_popup::CreateMenuPopup;
use crate::resource_downloader::app::popups::import_popup::ImportPopup;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::business::{Effect, SharedRDState};
use crate::resource_downloader::domain::{
    AppConfig, ListGroup, ListGroupLnk, ListLnk, ResourceType, SidebarItem,
};
use eframe::egui;
use egui::text::LayoutJob;
use egui::{Color32, StrokeKind, Ui};
use std::collections::{HashMap, HashSet};

type ListItem = (ListLnk, ResourceType, String, String, String, String, usize);

struct RenderContext<'a> {
    open_list: &'a Option<ListLnk>,
    _open_list_group: &'a Option<ListGroupLnk>,
    pending_sidebar_scroll: &'a Option<SidebarItem>,
    force_open_groups: HashSet<ListGroupLnk>,
}

impl<'a> RenderContext<'a> {
    fn new(
        open_list: &'a Option<ListLnk>,
        open_list_group: &'a Option<ListGroupLnk>,
        pending_sidebar_scroll: &'a Option<SidebarItem>,
        force_open_groups: HashSet<ListGroupLnk>,
    ) -> Self {
        Self {
            open_list,
            _open_list_group: open_list_group,
            pending_sidebar_scroll,
            force_open_groups,
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
    create_menu_popup: CreateMenuPopup,
    import_popup: ImportPopup,
    context_menu_target: Option<(ListLnk, egui::Rect)>,
    list_group_context_menu_target: Option<(ListGroupLnk, String, egui::Rect)>,
    import_options: ImportOptionsComponent,
    drop_targets: Vec<DropTarget>,
    hover_group_start: Option<(ListGroupLnk, f64)>,
    visible_items: Vec<SidebarItem>,
}

impl SidebarPanel {
    const DND_PAYLOAD_ID: &'static str = "dnd_last_payload";

    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_search_query: String::new(),
            create_menu_popup: CreateMenuPopup::new(state.clone()),
            import_popup: ImportPopup::new(state.clone()),
            context_menu_target: None,
            list_group_context_menu_target: None,
            import_options: ImportOptionsComponent::new(state.clone()),
            drop_targets: Vec::new(),
            hover_group_start: None,
            visible_items: Vec::new(),
        }
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.ash(|ui| {
            ui.ash_text_edit(&mut self.list_search_query, "🔍 Search Lists...");

            let offline_mode = self.state.read().offline_mode;
            ui.columns(2, |columns| {
                {
                    let ui = &mut columns[0];
                    ui.vertical_centered_justified(|ui| {
                        let mut create_btn = egui::Button::new(
                            egui::RichText::new("➕ Create").color(if offline_mode {
                                Color32::GRAY
                            } else {
                                Color32::LIGHT_GREEN
                            }),
                        );

                        if offline_mode {
                            create_btn =
                                create_btn.fill(Color32::from_rgba_unmultiplied(100, 100, 100, 50));
                        }

                        self.show_popup_button(
                            ui,
                            self.create_menu_popup.clone(),
                            !offline_mode,
                            |ui| {
                                ui.add(create_btn)
                                    .on_disabled_hover_text("Disabled in offline mode")
                            },
                        );
                    });
                }

                {
                    let ui = &mut columns[1];
                    ui.vertical_centered_justified(|ui| {
                        let mut import_btn = egui::Button::new(egui::RichText::new("📥 Import"));

                        if offline_mode {
                            import_btn =
                                import_btn.fill(Color32::from_rgba_unmultiplied(100, 100, 100, 50));
                        }

                        self.show_popup_button(
                            ui,
                            self.import_popup.clone(),
                            !offline_mode,
                            |ui| {
                                ui.add(import_btn)
                                    .on_disabled_hover_text("Disabled in offline mode")
                            },
                        );
                    });
                }
            });
        });

        ui.add_space(5.0);

        let (open_list, open_list_group, pending_sidebar_scroll) = {
            let mut state = self.state.write();
            (
                state.open_list.clone(),
                state.open_list_group.clone(),
                state.pending_sidebar_scroll.take(),
            )
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

        let (list_groups, list_group_assignments, sidebar_order) = {
            let state = self.state.read();
            let config = state.config.read();
            (
                config.list_groups.clone(),
                config.list_group_assignments.clone(),
                config.sidebar_ui_order.clone(),
            )
        };

        if is_searching {
            let query = self.list_search_query.to_lowercase();
            self.visible_items = list_items
                .iter()
                .filter(|item| item.3.to_lowercase().contains(&query))
                .map(|item| SidebarItem::List(item.0.clone()))
                .collect();
        } else {
            let list_group_map: HashMap<ListGroupLnk, ListGroup> = list_groups
                .iter()
                .map(|f| (f.lnk.clone(), f.clone()))
                .collect();

            let mut items_by_parent: HashMap<Option<ListGroupLnk>, Vec<SidebarItem>> =
                HashMap::new();
            for item in &sidebar_order {
                match item {
                    SidebarItem::List(l_lnk) => {
                        if list_group_assignments.contains_key(l_lnk)
                            || list_items.iter().any(|li| &li.0 == l_lnk)
                        {
                            let parent = list_group_assignments.get(l_lnk).cloned();
                            items_by_parent
                                .entry(parent)
                                .or_default()
                                .push(item.clone());
                        }
                    }
                    SidebarItem::ListGroup(lg_lnk) => {
                        if let Some(group) = list_group_map.get(lg_lnk) {
                            items_by_parent
                                .entry(group.parent_id.clone())
                                .or_default()
                                .push(item.clone());
                        }
                    }
                }
            }

            let mut flattened_visible = Vec::new();
            Self::collect_visible_items_recursive(
                None,
                &items_by_parent,
                &list_group_map,
                ui.ctx(),
                &mut flattened_visible,
            );
            self.visible_items = flattened_visible;
        }

        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            let selected: Vec<_> = self
                .state
                .read()
                .selected_sidebar_items
                .iter()
                .cloned()
                .collect();
            if !selected.is_empty() {
                ListActions::delete_items(self.state.clone(), selected);
                self.state.write().selected_sidebar_items.clear();
            }
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());

            if total_lists == 0 && list_groups.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label(egui::RichText::new("No lists yet!").weak());
                    ui.label(egui::RichText::new("You can easily import one:").weak());
                    ui.add_space(10.0);
                    self.import_options.render_contents(ui);
                });
                return;
            }

            let is_dragging = ui.input(|i| i.pointer.is_decidedly_dragging());
            let has_payload = Self::has_dnd_payload(ui.ctx());

            if is_dragging && has_payload {
                self.apply_smooth_scroll(ui);
                self.apply_edge_autoscroll(ui);
            }

            if is_searching {
                self.apply_edge_autoscroll(ui);
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
                    self.apply_row_interactions(ui, response, &item, &open_list);
                }
            } else {
                self.render_sidebar_content(
                    ui,
                    list_items,
                    &open_list,
                    &open_list_group,
                    &pending_sidebar_scroll,
                );
            }

            let bg_response = ui.allocate_response(ui.available_size(), egui::Sense::click());
            if bg_response.clicked() {
                self.state.write().selected_sidebar_items.clear();
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

        if let Some((list_group_lnk, list_group_name, rect)) = &self.list_group_context_menu_target
        {
            let menu = ListGroupContextMenu::new(
                self.state.clone(),
                list_group_lnk.clone(),
                list_group_name.clone(),
            );
            let menu_id = menu.id();
            let pm = self.state.read().popup_manager.clone();
            if pm.is_open(menu_id) {
                pm.register_interaction_area(menu_id, *rect);
                pm.request_show(Box::new(menu), *rect);
            } else {
                self.list_group_context_menu_target = None;
            }
        }
    }

    fn show_popup_button<P: Popup + Clone + 'static>(
        &self,
        ui: &mut Ui,
        popup: P,
        enabled: bool,
        add_button: impl FnOnce(&mut Ui) -> egui::Response,
    ) {
        let response = ui.add_enabled_ui(enabled, |ui| add_button(ui)).inner;
        let pm = self.state.read().popup_manager.clone();

        if response.clicked() {
            pm.toggle(popup.id());
        }
        pm.register_interaction_area(popup.id(), response.rect);
        pm.request_show(Box::new(popup), response.rect);
    }

    fn apply_smooth_scroll(&self, ui: &mut Ui) {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        if scroll_delta.y.abs() > 0.0 {
            ui.scroll_with_delta(scroll_delta);
        }
    }

    fn apply_edge_autoscroll(&self, ui: &mut Ui) {
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

    fn render_sidebar_content(
        &mut self,
        ui: &mut Ui,
        list_items: Vec<ListItem>,
        open_list: &Option<ListLnk>,
        open_list_group: &Option<ListGroupLnk>,
        pending_sidebar_scroll: &Option<SidebarItem>,
    ) {
        let (list_groups, list_group_assignments, mut sidebar_order) = {
            let state = self.state.read();
            let config = state.config.read();
            (
                config.list_groups.clone(),
                config.list_group_assignments.clone(),
                config.sidebar_ui_order.clone(),
            )
        };

        let list_group_map: HashMap<ListGroupLnk, ListGroup> = list_groups
            .iter()
            .map(|f| (f.lnk.clone(), f.clone()))
            .collect();

        let list_map: HashMap<ListLnk, ListItem> = list_items
            .iter()
            .map(|item| (item.0.clone(), item.clone()))
            .collect();

        if sidebar_order.is_empty() {
            let mut new_order = Vec::new();

            for list_group in &list_groups {
                new_order.push(SidebarItem::from(&list_group.lnk));
            }
            for list in &list_items {
                new_order.push(SidebarItem::from(&list.0));
            }

            sidebar_order = new_order.clone();

            ListActions::set_sidebar_ui_order(self.state.clone(), new_order);
        }

        let mut changed = false;
        for list_group in &list_groups {
            if !sidebar_order.contains(&SidebarItem::from(&list_group.lnk)) {
                sidebar_order.push(SidebarItem::from(&list_group.lnk));
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
        let force_open_groups = self.collect_force_open_groups(
            &list_group_map,
            &list_group_assignments,
            pending_sidebar_scroll,
        );

        let mut items_by_parent: HashMap<Option<ListGroupLnk>, Vec<SidebarItem>> = HashMap::new();

        for item in &sidebar_order {
            match item {
                SidebarItem::List(l_lnk) => {
                    if list_map.contains_key(l_lnk) {
                        let parent = list_group_assignments.get(l_lnk).cloned();
                        items_by_parent
                            .entry(parent)
                            .or_default()
                            .push(item.clone());
                    }
                }
                SidebarItem::ListGroup(lg_lnk) => {
                    if let Some(group) = list_group_map.get(lg_lnk) {
                        items_by_parent
                            .entry(group.parent_id.clone())
                            .or_default()
                            .push(item.clone());
                    }
                }
            }
        }

        let render_ctx = RenderContext::new(
            open_list,
            open_list_group,
            pending_sidebar_scroll,
            force_open_groups,
        );

        self.drop_targets.clear();

        let is_dragging = ui.input(|i| i.pointer.is_decidedly_dragging());
        let pointer_pos = ui.ctx().pointer_hover_pos();

        self.render_mixed_list(
            ui,
            &items_by_parent,
            &list_group_map,
            &list_map,
            None,
            &render_ctx,
            0,
        );

        let has_payload = Self::has_dnd_payload(ui.ctx());

        if is_dragging
            && has_payload
            && let Some(pos) = pointer_pos
            && let Some(closest) = self.find_closest_target(pos)
        {
            self.visualize_drop_target(ui, &closest);
        }

        let was_dragging = Self::has_dnd_payload(ui.ctx());

        if was_dragging
            && ui.input(|i| i.pointer.any_released())
            && let Some(pos) = pointer_pos
        {
            let payload_opt = Self::get_dnd_payload(ui.ctx());

            if let Some(payload) = payload_opt {
                if let Some(closest) = self.find_closest_target(pos) {
                    self.apply_drop(&closest, &payload);
                }

                Self::clear_dnd_payload(ui.ctx());
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_mixed_list(
        &mut self,
        ui: &mut Ui,
        items_by_parent: &HashMap<Option<ListGroupLnk>, Vec<SidebarItem>>,
        list_group_map: &HashMap<ListGroupLnk, ListGroup>,
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
                    if let Some(list_group) = list_group_map.get(lg_lnk) {
                        self.render_list_group_new(
                            ui,
                            list_group.clone(),
                            items_by_parent,
                            list_group_map,
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
        list_group_map: &HashMap<ListGroupLnk, ListGroup>,
        list_map: &HashMap<ListLnk, ListItem>,
        ctx: &RenderContext,
        depth: usize,
        is_last_in_group: bool,
    ) {
        let lg_lnk = list_group.lnk.clone();
        let id = egui::Id::new("sidebar_list_group").with(&list_group.lnk);
        let payload = format!("list_group:{}", list_group.lnk);

        if ui.ctx().is_being_dragged(id) {
            ui.ctx().memory_mut(|mem| {
                mem.data
                    .insert_temp(egui::Id::new(Self::DND_PAYLOAD_ID), payload.clone());
            });
        }

        let mut collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            id,
            !list_group.collapsed,
        );

        if ctx.force_open_groups.contains(&list_group.lnk) && !collapsing.is_open() {
            collapsing.set_open(true);
        }

        let mut arrow_clicked = false;

        let is_selected = {
            let state = self.state.read();
            state
                .selected_sidebar_items
                .contains(&SidebarItem::ListGroup(list_group.lnk.clone()))
        };

        let frame = ui
            .ash_selectable_frame(is_selected)
            .inner_margin(egui::Margin::symmetric(4, 1));

        let list_group_response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let arrow = ui.add(
                        egui::Button::new(if collapsing.is_open() { "➖" } else { "➕" })
                            .frame(false)
                            .small(),
                    );
                    if arrow.clicked() {
                        arrow_clicked = true;
                    }
                    let inner_response = ui.dnd_drag_source(id, payload.clone(), |ui| {
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            let icon = if list_group.is_instance { "🎮 " } else { "" };
                            let mut job = LayoutJob::default();

                            job.append(
                                icon,
                                0.0,
                                egui::TextFormat {
                                    color: Color32::LIGHT_BLUE,
                                    ..Default::default()
                                },
                            );

                            job.append(
                                &list_group.name,
                                0.0,
                                egui::TextFormat {
                                    ..Default::default()
                                },
                            );

                            ui.add(egui::Label::new(job).selectable(false));

                            if !collapsing.is_open() {
                                let list_count =
                                    Self::count_lists_recursive(&lg_lnk, items_by_parent);
                                if list_count > 0 {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let highlight_color = Color32::from_rgb(230, 230, 230);
                                            egui::Frame::default()
                                                .fill(highlight_color.gamma_multiply(0.1))
                                                .stroke(egui::Stroke::new(
                                                    1.0,
                                                    highlight_color.gamma_multiply(0.2),
                                                ))
                                                .corner_radius(3)
                                                .inner_margin(egui::Margin::symmetric(5, 0))
                                                .show(ui, |ui| {
                                                    ui.add(egui::Label::new(
                                                        egui::RichText::new(format!(
                                                            "{}",
                                                            list_count
                                                        ))
                                                        .color(highlight_color)
                                                        .small(),
                                                    ));
                                                });
                                        },
                                    );
                                }
                            }
                        })
                        .response
                    });

                    let drag_response = inner_response
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                    if Self::has_dnd_payload(ui.ctx()) {
                        let is_hovered = ui.rect_contains_pointer(drag_response.rect);
                        if is_hovered && !collapsing.is_open() {
                            let current_time = ui.input(|i| i.time);
                            if let Some((ref h_lnk, start_time)) = self.hover_group_start {
                                if h_lnk == &lg_lnk {
                                    if current_time - start_time > 0.5 {
                                        collapsing.set_open(true);
                                        ListGroupActions::toggle_list_group_collapsed(
                                            self.state.clone(),
                                            lg_lnk.clone(),
                                        );
                                        self.hover_group_start = None;
                                    }
                                } else {
                                    self.hover_group_start = Some((lg_lnk.clone(), current_time));
                                }
                            } else {
                                self.hover_group_start = Some((lg_lnk.clone(), current_time));
                            }
                        } else if let Some((ref h_lnk, _)) = self.hover_group_start
                            && h_lnk == &lg_lnk
                        {
                            self.hover_group_start = None;
                        }
                    }

                    if drag_response.clicked() && !arrow_clicked {
                        let modifiers = ui.input(|i| i.modifiers);
                        self.handle_sidebar_item_click(
                            SidebarItem::ListGroup(lg_lnk.clone()),
                            modifiers,
                        );
                    }

                    if drag_response.secondary_clicked() {
                        {
                            let mut state = self.state.write();
                            let item_sid = SidebarItem::ListGroup(lg_lnk.clone());
                            if !state.selected_sidebar_items.contains(&item_sid) {
                                state.selected_sidebar_items.clear();
                                state.selected_sidebar_items.insert(item_sid.clone());
                                state.last_clicked_sidebar_item = Some(item_sid);
                            }
                        }
                        self.list_group_context_menu_target =
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
                .inner
            })
            .inner;

        if let Some(Some(lg_lnk)) = ctx
            .pending_sidebar_scroll
            .clone()
            .map(|s| s.list_group_lnk())
            && lg_lnk == list_group.lnk
        {
            list_group_response.scroll_to_me(Some(egui::Align::Center));
        }

        self.drop_targets.push(DropTarget {
            dnd_id: list_group.lnk.to_context_id(),
            rect: list_group_response.rect,
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
                    list_group_map,
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

        if is_last_in_group && list_group.parent_id.is_none() {
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
        if let Some(stripped) = payload.strip_prefix("list_group:") {
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
            if let Some(list_group) = config
                .list_groups
                .iter_mut()
                .find(|f| item.match_list_group(&f.lnk))
            {
                list_group.parent_id = new_parent_lg.clone();
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
        state_guard.dispatch(Effect::SaveConfig { config });
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
                    .insert_temp(egui::Id::new(Self::DND_PAYLOAD_ID), payload.clone());
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

        self.apply_row_interactions(ui, drag_response, item, open_list);
    }

    fn render_row(
        &self,
        ui: &mut Ui,
        item: &ListItem,
        _open_list: &Option<ListLnk>,
        pending_list_scroll: &Option<ListLnk>,
    ) -> egui::Response {
        let (list, resource_type, icon, name, version, loader, count) = item;
        let should_scroll = pending_list_scroll.as_ref().is_some_and(|l| l == list);

        let is_selected = {
            let state = self.state.read();
            state
                .selected_sidebar_items
                .contains(&SidebarItem::List(list.clone()))
        };
        let frame = ui
            .ash_selectable_frame(is_selected)
            .inner_margin(egui::Margin {
                left: 8,
                right: 8,
                top: 6,
                bottom: 6,
            });

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

                        let highlight_color = Color32::from_rgb(230, 230, 230);
                        egui::Frame::default()
                            .fill(highlight_color.gamma_multiply(0.1))
                            .stroke(egui::Stroke::new(1.0, highlight_color.gamma_multiply(0.2)))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{count}"))
                                        .color(highlight_color)
                                        .small(),
                                );
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
        let (item_id, is_list_group_drag) = self.parse_payload(payload);
        let (actual_target_id, is_inside_end) = Self::normalize_drop_target(&closest.target.dnd_id);

        if item_id == actual_target_id {
            return;
        }

        let sidebar_item = self.sidebar_item_by_context_id(&item_id);
        let actual_target_item = self.sidebar_item_by_context_id(&actual_target_id);

        if self.is_descendant_move(is_list_group_drag, &closest.target.parent_id, &sidebar_item) {
            return;
        }

        if is_inside_end {
            if is_list_group_drag {
                let target_as_listgroup = ListGroupLnk::from(actual_target_id.clone());
                if self.is_descendant_of(&target_as_listgroup, &sidebar_item) {
                    return;
                }
            }

            let state = self.state.read();
            let config = state.config.read();

            let current_parent = if is_list_group_drag {
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

            let target_children = Self::items_in_parent_context_id(&config, &actual_target_id);

            if current_parent.map(|a| a.to_context_id()) == Some(actual_target_id.clone())
                && target_children.last().map(|a| a.to_context_id()) == Some(item_id.clone())
            {
                drop(config);
                drop(state);
                return;
            }

            drop(config);
            drop(state);

            let insert_after = target_children.last().cloned();

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
                    if self.is_descendant_move(
                        is_list_group_drag,
                        &closest.target.parent_id,
                        &sidebar_item,
                    ) {
                        return;
                    }

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
                    .filter(|item| match item {
                        SidebarItem::List(l_lnk) => {
                            config.list_group_assignments.get(l_lnk).cloned()
                                == closest.target.parent_id
                        }
                        SidebarItem::ListGroup(lg_lnk) => {
                            let item_parent = if let Some(list_group) = config
                                .list_groups
                                .iter()
                                .find(|f| f.lnk.clone() == lg_lnk.clone())
                            {
                                list_group.parent_id.clone()
                            } else {
                                None
                            };
                            item_parent == closest.target.parent_id
                        }
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

                if self.is_descendant_move(
                    is_list_group_drag,
                    &closest.target.parent_id,
                    &sidebar_item,
                ) {
                    return;
                }

                self.move_item(sidebar_item, closest.target.parent_id.clone(), insert_after);
            }
            DropPosition::After => {
                if self.is_descendant_move(
                    is_list_group_drag,
                    &closest.target.parent_id,
                    &sidebar_item,
                ) {
                    return;
                }

                let state = self.state.read();
                let config = state.config.read();

                let siblings_in_order = Self::items_in_parent(&config, &closest.target.parent_id);

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

    fn collect_force_open_groups(
        &self,
        list_group_map: &HashMap<ListGroupLnk, ListGroup>,
        list_group_assignments: &HashMap<ListLnk, ListGroupLnk>,
        pending_sidebar_scroll: &Option<SidebarItem>,
    ) -> HashSet<ListGroupLnk> {
        let Some(target) = pending_sidebar_scroll else {
            return HashSet::new();
        };

        let mut groups = HashSet::new();
        let mut current = match target {
            SidebarItem::List(list_lnk) => list_group_assignments.get(list_lnk).cloned(),
            SidebarItem::ListGroup(list_group_lnk) => Some(list_group_lnk.clone()),
        };

        while let Some(lg) = current {
            groups.insert(lg.clone());
            current = list_group_map.get(&lg).and_then(|g| g.parent_id.clone());
        }

        groups
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
        while let Some(list_group) = config.list_groups.iter().find(|f| f.lnk == *current) {
            if let Some(parent_id) = &list_group.parent_id {
                if potential_ancestor.match_list_group(parent_id) {
                    return true;
                }
                current = parent_id;
            } else {
                break;
            }
        }

        false
    }

    fn is_descendant_move(
        &self,
        is_list_group_drag: bool,
        target_parent: &Option<ListGroupLnk>,
        sidebar_item: &SidebarItem,
    ) -> bool {
        is_list_group_drag
            && target_parent
                .as_ref()
                .is_some_and(|parent| self.is_descendant_of(parent, sidebar_item))
    }

    fn normalize_drop_target(dnd_id: &str) -> (String, bool) {
        if dnd_id.ends_with("_inside_end") {
            (dnd_id.trim_end_matches("_inside_end").to_string(), true)
        } else if dnd_id.ends_with("_after") {
            (dnd_id.trim_end_matches("_after").to_string(), false)
        } else {
            (dnd_id.to_string(), false)
        }
    }

    fn sidebar_item_by_context_id(&self, id: &str) -> SidebarItem {
        self.state
            .read()
            .config
            .read()
            .sidebar_ui_order
            .iter()
            .find(|i| i.to_context_id() == id)
            .unwrap()
            .clone()
    }

    fn get_dnd_payload(ctx: &egui::Context) -> Option<String> {
        ctx.memory(|mem| {
            mem.data
                .get_temp::<String>(egui::Id::new(Self::DND_PAYLOAD_ID))
        })
    }

    fn has_dnd_payload(ctx: &egui::Context) -> bool {
        Self::get_dnd_payload(ctx).is_some()
    }

    fn clear_dnd_payload(ctx: &egui::Context) {
        ctx.memory_mut(|mem| {
            mem.data
                .remove::<String>(egui::Id::new(Self::DND_PAYLOAD_ID));
        });
    }

    fn apply_row_interactions(
        &mut self,
        ui: &mut Ui,
        response: egui::Response,
        item: &ListItem,
        _open_list: &Option<ListLnk>,
    ) {
        if response.hovered() {
            ui.painter().rect_stroke(
                response.rect,
                4.0,
                egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),
                StrokeKind::Middle,
            );
        }

        if response.clicked() {
            let modifiers = ui.input(|i| i.modifiers);
            self.handle_sidebar_item_click(SidebarItem::List(item.0.clone()), modifiers);
        }

        if let Some((target_lnk, _)) = &self.context_menu_target
            && target_lnk == &item.0
        {
            self.context_menu_target = Some((item.0.clone(), response.rect));
        }
        if response.secondary_clicked() {
            {
                let mut state = self.state.write();
                let item_sid = SidebarItem::List(item.0.clone());
                if !state.selected_sidebar_items.contains(&item_sid) {
                    state.selected_sidebar_items.clear();
                    state.selected_sidebar_items.insert(item_sid.clone());
                    state.last_clicked_sidebar_item = Some(item_sid);
                }
            }
            self.context_menu_target = Some((item.0.clone(), response.rect));
            let menu_id = egui::Id::new("list_context_menu").with(&item.0);
            self.state.read().popup_manager.toggle(menu_id);
        }
    }

    fn item_parent(config: &AppConfig, id: &SidebarItem) -> Option<ListGroupLnk> {
        match id {
            SidebarItem::List(l_lnk) => config.list_group_assignments.get(l_lnk).cloned(),
            SidebarItem::ListGroup(lg_lnk) => {
                if let Some(list_group) = config
                    .list_groups
                    .iter()
                    .find(|f| f.lnk.clone() == lg_lnk.clone())
                {
                    list_group.parent_id.clone()
                } else {
                    None
                }
            }
        }
    }

    fn items_in_parent(config: &AppConfig, parent_id: &Option<ListGroupLnk>) -> Vec<SidebarItem> {
        config
            .sidebar_ui_order
            .iter()
            .filter(|id| Self::item_parent(config, id) == *parent_id)
            .cloned()
            .collect()
    }

    fn items_in_parent_context_id(config: &AppConfig, parent_id: &str) -> Vec<SidebarItem> {
        config
            .sidebar_ui_order
            .iter()
            .filter(|id| {
                Self::item_parent(config, id).map(|a| a.to_context_id())
                    == Some(parent_id.to_string())
            })
            .cloned()
            .collect()
    }

    fn handle_sidebar_item_click(&self, item: SidebarItem, modifiers: egui::Modifiers) {
        let mut state = self.state.write();

        if modifiers.shift {
            if let Some(last) = &state.last_clicked_sidebar_item {
                let flattened = &self.visible_items;
                if let (Some(start_idx), Some(end_idx)) = (
                    flattened.iter().position(|i| i == last),
                    flattened.iter().position(|i| i == &item),
                ) {
                    if !modifiers.ctrl && !modifiers.command {
                        state.selected_sidebar_items.clear();
                    }

                    let range = if start_idx < end_idx {
                        start_idx..=end_idx
                    } else {
                        end_idx..=start_idx
                    };

                    for idx in range {
                        state.selected_sidebar_items.insert(flattened[idx].clone());
                    }
                } else {
                    state.selected_sidebar_items.insert(item);
                }
            } else {
                state.selected_sidebar_items.insert(item);
            }
        } else if modifiers.command || modifiers.ctrl {
            if state.selected_sidebar_items.contains(&item) {
                state.selected_sidebar_items.remove(&item);
            } else {
                state.selected_sidebar_items.insert(item.clone());
            }
            state.last_clicked_sidebar_item = Some(item);
        } else {
            state.selected_sidebar_items.clear();
            state.selected_sidebar_items.insert(item.clone());
            state.last_clicked_sidebar_item = Some(item.clone());

            match &item {
                SidebarItem::List(l_lnk) => {
                    drop(state);
                    ListActions::toggle_open_list(self.state.clone(), l_lnk);
                }
                SidebarItem::ListGroup(lg_lnk) => {
                    state.set_open_list_group(Some(lg_lnk.clone()));
                }
            }
        }
    }

    fn collect_visible_items_recursive(
        parent_id: Option<ListGroupLnk>,
        items_by_parent: &HashMap<Option<ListGroupLnk>, Vec<SidebarItem>>,
        list_group_map: &HashMap<ListGroupLnk, ListGroup>,
        ctx: &egui::Context,
        out: &mut Vec<SidebarItem>,
    ) {
        if let Some(items) = items_by_parent.get(&parent_id) {
            for item in items {
                out.push(item.clone());
                if let SidebarItem::ListGroup(lg_lnk) = item {
                    let id = egui::Id::new("sidebar_list_group").with(lg_lnk);
                    let is_open = if let Some(group) = list_group_map.get(lg_lnk) {
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ctx,
                            id,
                            !group.collapsed,
                        )
                        .is_open()
                    } else {
                        false
                    };

                    if is_open {
                        Self::collect_visible_items_recursive(
                            Some(lg_lnk.clone()),
                            items_by_parent,
                            list_group_map,
                            ctx,
                            out,
                        );
                    }
                }
            }
        }
    }

    fn count_lists_recursive(
        group_lnk: &ListGroupLnk,
        items_by_parent: &HashMap<Option<ListGroupLnk>, Vec<SidebarItem>>,
    ) -> usize {
        let mut count = 0;
        if let Some(items) = items_by_parent.get(&Some(group_lnk.clone())) {
            for item in items {
                match item {
                    SidebarItem::List(_) => count += 1,
                    SidebarItem::ListGroup(sub_lnk) => {
                        count += Self::count_lists_recursive(sub_lnk, items_by_parent);
                    }
                }
            }
        }
        count
    }
}
