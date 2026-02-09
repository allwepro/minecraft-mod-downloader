use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::modals::create_modal::CreateModal;
use crate::resource_downloader::app::popups::import_popup::ImportPopup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{ListLnk, ResourceType};
use crate::{get_list, get_list_type};
use eframe::egui;
use egui::{Color32, StrokeKind, Ui};

pub struct SidebarPanel {
    state: SharedRDState,
    list_search_query: String,
    new_list_modal: CreateModal,
    import_popup: ImportPopup,
}

impl SidebarPanel {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            state: state.clone(),
            list_search_query: String::new(),
            new_list_modal: CreateModal::new(state.clone()),
            import_popup: ImportPopup::new(state.clone()),
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
                self.state
                    .read()
                    .submit_modal(Box::new(self.new_list_modal.clone()));
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

        let mut list_info: Vec<(ListLnk, String, String, String, String)> = {
            let state = self.state.read();
            let query = self.list_search_query.to_lowercase();
            state.list_pool.map_filter(|list| {
                if !query.is_empty() && !list.get_name().to_lowercase().contains(&query) {
                    return None;
                }

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
                    type_icon,
                    list.get_name(),
                    version_display,
                    loader_display,
                ))
            })
        };

        list_info.sort_by(|a, b| a.2.to_lowercase().cmp(&b.2.to_lowercase()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (list, icon, name, version, loader) in list_info {
                let is_selected = open_list.clone().is_some_and(|l| l == list);
                let should_scroll = pending_list_scroll.as_ref().is_some_and(|l| l == &list);

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
                                ui.add(
                                    egui::Label::new(egui::RichText::new(name).strong()).truncate(),
                                );
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
                            });
                        });
                    })
                    .response;

                if should_scroll {
                    response.scroll_to_me(Some(egui::Align::Center));
                }

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
                    let next_state = if is_selected {
                        None
                    } else {
                        let list_type = get_list_type!(self.state, &list);
                        let dir = get_list!(self.state, &list)
                            .read()
                            .get_resource_type_config(&list_type)
                            .expect("List without type")
                            .download_dir
                            .clone();
                        Some((list_type, dir))
                    };

                    let mut state = self.state.write();
                    state.found_files = None;
                    state.download_status.clear();

                    if let Some((lt, dir)) = next_state {
                        state.find_files(dir.parse().unwrap(), lt.file_extension());
                        state.set_open_list(Some(list.clone()));
                    } else {
                        state.set_open_list(None);
                    }
                }
            }
        });
    }
}
