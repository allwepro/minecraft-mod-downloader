use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::modals::create_modal::CreateModal;
use crate::resource_downloader::app::popups::import_popup::ImportPopup;
use crate::resource_downloader::app::popups::list_context_menu::ListContextMenu;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::domain::{ListLnk, ResourceType};
use eframe::egui;
use egui::{Color32, StrokeKind, Ui};
use std::collections::{HashMap, HashSet};

pub struct SidebarPanel {
    state: SharedRDState,
    list_search_query: String,
    new_list_modal: CreateModal,
    import_popup: ImportPopup,
    context_menu_target: Option<(ListLnk, egui::Rect)>,
}

impl SidebarPanel {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_search_query: String::new(),
            new_list_modal: CreateModal::new(state.clone()),
            import_popup: ImportPopup::new(state.clone()),
            context_menu_target: None,
        }
    }

    pub fn show(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.list_search_query)
                .hint_text("🔍 Search Lists...")
                .desired_width(ui.available_width()),
        );

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let button_width = ui.available_width() - 35.0;
            if ui
                .add_sized(
                    [button_width, 25.0],
                    egui::Button::new(
                        egui::RichText::new("➕ New List").color(Color32::LIGHT_GREEN),
                    ),
                )
                .clicked()
            {
                let mm = self.state.read().modal_manager.clone();
                mm.open(Box::new(self.new_list_modal.clone()));
            }

            let import_btn = ui
                .add_sized([25.0, 25.0], egui::Button::new("📥"))
                .on_hover_text("Import");

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

        let mut list_items: Vec<(ListLnk, ResourceType, String, String, String, String, usize)> = {
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

        let is_searching = !self.list_search_query.is_empty();

        if is_searching {
            let query = self.list_search_query.to_lowercase();
            list_items.retain(|item| item.2.to_lowercase().contains(&query));
            list_items.sort_by(|a, b| a.2.to_lowercase().cmp(&b.2.to_lowercase()));
        } else {
            let (changed, order) = {
                let state = self.state.read();
                let config = state.config.read();
                let mut order = config.list_order.clone();

                let available: HashSet<String> =
                    list_items.iter().map(|i| i.0.to_string()).collect();
                let mut changed = false;

                let initial_len = order.len();
                order.retain(|id| available.contains(id));
                if order.len() != initial_len {
                    changed = true;
                }

                let current_order_set: HashSet<String> = order.iter().cloned().collect();
                let mut new_items: Vec<_> = list_items
                    .iter()
                    .filter(|i| !current_order_set.contains(&i.0.to_string()))
                    .collect();

                if !new_items.is_empty() {
                    changed = true;
                    new_items.sort_by(|a, b| a.2.to_lowercase().cmp(&b.2.to_lowercase()));
                    for item in new_items {
                        order.push(item.0.to_string());
                    }
                }
                (changed, order)
            };

            if changed {
                ListActions::set_list_order(self.state.clone(), order.clone());
            }

            let mut item_map: HashMap<String, _> = list_items
                .into_iter()
                .map(|i| (i.0.to_string(), i))
                .collect();
            list_items = order
                .into_iter()
                .filter_map(|id| item_map.remove(&id))
                .collect();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());

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
                for item in list_items {
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
                let mut moved_item = None;
                for (idx, item) in list_items.iter().enumerate() {
                    let item_id = ui.make_persistent_id("list_dnd").with(&item.0);
                    let payload = idx;

                    let inner_response = ui.dnd_drag_source(item_id, payload, |ui| {
                        self.render_row(ui, item, &open_list, &pending_list_scroll)
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
                        self.handle_list_click(&item.0, &open_list);
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

                    if let Some(source_idx) = inner_response.response.dnd_release_payload::<usize>()
                    {
                        moved_item = Some((*source_idx, idx));
                    }
                }

                if ui.input(|i| i.pointer.is_decidedly_dragging()) {
                    let unused_height =
                        ui.clip_rect().max.y - ui.cursor().min.y - ui.spacing().item_spacing.y;
                    let remaining_height = unused_height.max(50.0);
                    let spacer = ui.allocate_response(
                        egui::vec2(ui.available_width(), remaining_height),
                        egui::Sense::hover(),
                    );

                    if let Some(source_idx) = spacer.dnd_release_payload::<usize>() {
                        moved_item = Some((*source_idx, list_items.len()));
                    }
                }

                if let Some((from_idx, mut to_idx)) = moved_item
                    && from_idx != to_idx
                {
                    if from_idx < to_idx {
                        to_idx -= 1;
                    }
                    let moved = list_items.remove(from_idx);
                    list_items.insert(to_idx, moved);

                    let new_order: Vec<String> =
                        list_items.iter().map(|i| i.0.to_string()).collect();

                    ListActions::set_list_order(self.state.clone(), new_order);
                }
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
