use crate::common::ui::ash_ui::AshUi;
use crate::common::ui::structs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use crate::resource_downloader::business::list_group_actions::ListGroupActions;
use crate::resource_downloader::domain::{ListLnk, SidebarItem};
use eframe::egui;
use egui::{Color32, Id, Ui};

#[derive(Clone)]
pub struct ListContextMenu {
    state: SharedRDState,
    list_lnk: ListLnk,
}

impl ListContextMenu {
    pub fn new(state: SharedRDState, list_lnk: ListLnk) -> Self {
        Self { state, list_lnk }
    }
}

impl Popup for ListContextMenu {
    fn id(&self) -> Id {
        Id::new("list_context_menu").with(&self.list_lnk)
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        let list_lnk = self.list_lnk.clone();

        ui.ash_context_menu(|ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                let (current_lg_lnk, list_groups) = {
                    let state = self.state.read();
                    let config = state.config.read();
                    let current = config.list_group_assignments.get(&list_lnk).cloned();
                    (current, config.list_groups.clone())
                };

                if !list_groups.is_empty() {
                    ui.menu_button("📁  Move to Group", |ui| {
                        ui.set_min_width(180.0);

                        let is_in_no_lg = current_lg_lnk.is_none();
                        let no_lg_text = if is_in_no_lg {
                            egui::RichText::new("✅ No Group").strong()
                        } else {
                            egui::RichText::new("   No Group")
                        };

                        if ui.button(no_lg_text).clicked() {
                            ListGroupActions::move_list_to_list_group(
                                self.state.clone(),
                                list_lnk.clone(),
                                None,
                            );
                            *open = false;
                        }

                        ui.separator();

                        for list_group in list_groups {
                            let is_current = current_lg_lnk.as_ref() == Some(&list_group.lnk);

                            let lg_text = if is_current {
                                egui::RichText::new(format!("✅ {}", list_group.name)).strong()
                            } else {
                                egui::RichText::new(format!("   {}", list_group.name))
                            };

                            let mut button = egui::Button::new(lg_text);

                            if is_current {
                                button =
                                    button.fill(Color32::from_rgba_unmultiplied(100, 150, 200, 50));
                            }

                            if ui.add(button).clicked() {
                                if !is_current {
                                    ListGroupActions::move_list_to_list_group(
                                        self.state.clone(),
                                        list_lnk.clone(),
                                        Some(list_group.lnk),
                                    );
                                }
                                *open = false;
                            }
                        }
                    });

                    ui.separator();
                }

                if ui.button("📂  Open Folder").clicked() {
                    ListActions::open_folder(self.state.clone(), list_lnk.clone());
                    *open = false;
                }

                if ui.button("👥  Duplicate").clicked() {
                    ListActions::duplicate_list(self.state.clone(), list_lnk.clone());
                    *open = false;
                }

                ui.separator();

                let delete_btn = egui::Button::new(
                    egui::RichText::new("🗑  Delete").color(Color32::from_rgb(255, 100, 100)),
                );

                if ui.add(delete_btn).clicked() {
                    let selected_items = {
                        let state = self.state.read();
                        if state
                            .selected_sidebar_items
                            .contains(&SidebarItem::List(list_lnk.clone()))
                        {
                            state
                                .selected_sidebar_items
                                .iter()
                                .cloned()
                                .collect::<Vec<_>>()
                        } else {
                            vec![SidebarItem::List(list_lnk.clone())]
                        }
                    };

                    ListActions::delete_items(self.state.clone(), selected_items);
                    self.state.write().selected_sidebar_items.clear();
                    *open = false;
                }
            });
        });
    }
}
