use crate::common::ui::ash_ui::AshUi;
use crate::common::ui::structs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::domain::{ListGroupLnk, SidebarItem};
use eframe::egui;
use egui::{Color32, Id, Ui};
use std::collections::HashSet;

#[derive(Clone)]
pub struct MultiSelectContextMenu {
    state: SharedRDState,
    selected_items: Vec<SidebarItem>,
}

impl MultiSelectContextMenu {
    pub fn new(
        state: SharedRDState,
        selected_items: Vec<SidebarItem>,
        _trigger_rect: egui::Rect,
    ) -> Self {
        Self {
            state,
            selected_items,
        }
    }
}

impl Popup for MultiSelectContextMenu {
    fn id(&self) -> Id {
        Id::new("multiselect_context_menu")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        ui.ash_context_menu(|ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} items selected", self.selected_items.len()))
                        .weak()
                        .small(),
                );
                ui.separator();

                let all_same_parent = self.check_all_same_parent();

                if all_same_parent {
                    let list_groups = {
                        let state = self.state.read();
                        let config = state.config.read();
                        config.list_groups.clone()
                    };

                    if !list_groups.is_empty() {
                        ui.menu_button("📁  Move to Group", |ui| {
                            ui.set_min_width(180.0);

                            let current_parent = self.get_common_parent();

                            let is_in_no_lg = current_parent.is_none();
                            let no_lg_text = if is_in_no_lg {
                                egui::RichText::new("✅ No Group").strong()
                            } else {
                                egui::RichText::new("   No Group")
                            };

                            if ui.button(no_lg_text).clicked() {
                                for item in &self.selected_items {
                                    if let SidebarItem::List(list_lnk) = item {
                                        ListGroupActions::move_list_to_list_group(
                                            self.state.clone(),
                                            list_lnk.clone(),
                                            None,
                                        );
                                    }
                                }
                                *open = false;
                            }

                            ui.separator();

                            for list_group in list_groups {
                                let is_current = current_parent.as_ref() == Some(&list_group.lnk);

                                let lg_text = if is_current {
                                    egui::RichText::new(format!("✅ {}", list_group.name)).strong()
                                } else {
                                    egui::RichText::new(format!("   {}", list_group.name))
                                };

                                let mut button = egui::Button::new(lg_text);

                                if is_current {
                                    button = button
                                        .fill(Color32::from_rgba_unmultiplied(100, 150, 200, 50));
                                }

                                if ui.add(button).clicked() {
                                    if !is_current {
                                        for item in &self.selected_items {
                                            if let SidebarItem::List(list_lnk) = item {
                                                ListGroupActions::move_list_to_list_group(
                                                    self.state.clone(),
                                                    list_lnk.clone(),
                                                    Some(list_group.lnk.clone()),
                                                );
                                            }
                                        }
                                    }
                                    *open = false;
                                }
                            }
                        });

                        ui.separator();
                    }
                }

                let delete_btn = egui::Button::new(
                    egui::RichText::new("🗑  Delete").color(Color32::from_rgb(255, 100, 100)),
                );

                if ui.add(delete_btn).clicked() {
                    ListActions::delete_items(self.state.clone(), self.selected_items.clone());
                    self.state.write().selected_sidebar_items.clear();
                    *open = false;
                }
            });
        });
    }
}

impl MultiSelectContextMenu {
    fn check_all_same_parent(&self) -> bool {
        let state = self.state.read();
        let config = state.config.read();

        let parents: HashSet<Option<ListGroupLnk>> = self
            .selected_items
            .iter()
            .map(|item| match item {
                SidebarItem::List(list_lnk) => config.list_group_assignments.get(list_lnk).cloned(),
                SidebarItem::ListGroup(lg_lnk) => config
                    .list_groups
                    .iter()
                    .find(|lg| &lg.lnk == lg_lnk)
                    .and_then(|lg| lg.parent_id.clone()),
            })
            .collect();

        parents.len() == 1
    }

    fn get_common_parent(&self) -> Option<ListGroupLnk> {
        let state = self.state.read();
        let config = state.config.read();

        if self.selected_items.is_empty() {
            return None;
        }

        match &self.selected_items[0] {
            SidebarItem::List(list_lnk) => config.list_group_assignments.get(list_lnk).cloned(),
            SidebarItem::ListGroup(lg_lnk) => config
                .list_groups
                .iter()
                .find(|lg| &lg.lnk == lg_lnk)
                .and_then(|lg| lg.parent_id.clone()),
        }
    }
}
