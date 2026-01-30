use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::app::modals::create_modal::CreateModal;
use crate::resource_downloader::app::popups::import_popup::ImportPopup;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{ListLnk, ResourceType};
use crate::{get_list, get_list_type};
use eframe::egui;
use egui::Ui;

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
                .hint_text("🔍 Search lists...")
                .desired_width(ui.available_width()),
        );

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let button_width = ui.available_width() - 35.0;
            if ui
                .add_sized([button_width, 25.0], egui::Button::new("➕ New List"))
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

        ui.add_space(4.0);
        ui.separator();

        let open_list = { self.state.read().open_list.clone() };

        let mut list_info: Vec<(ListLnk, String)> = {
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
                    .get_resource_type(&resource_type)
                    .map(|c| c.get_loader().name.clone())
                    .unwrap_or("Unknown".parse().unwrap());
                let version_display = list.get_game_version().name;

                Some((
                    list.get_lnk(),
                    format!(
                        "{} {} [{} | {}]",
                        type_icon,
                        list.get_name(),
                        version_display,
                        loader_display
                    ),
                ))
            })
        };

        list_info.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (list, list_name) in list_info {
                let is_selected = open_list.clone().is_some_and(|l| l == list);
                let button = egui::Button::new(list_name)
                    .selected(is_selected)
                    .frame_when_inactive(is_selected)
                    .frame(true)
                    .right_text("");
                if ui.add_sized([ui.available_width(), 0.0], button).clicked() {
                    self.state.write().found_files = None;
                    self.state.write().download_status.clear();
                    if let Some(list) = self.state.read().open_list.clone() {
                        self.state.read().list_pool.save(&list);
                    }
                    if is_selected {
                        self.state.write().open_list = None;
                    } else {
                        let list_type = { get_list_type!(self.state, &list) };
                        let dir = {
                            get_list!(self.state, &list)
                                .read()
                                .get_resource_type(&list_type)
                                .expect("List without type")
                                .download_dir
                                .clone()
                        };
                        self.state
                            .write()
                            .find_files(dir.parse().unwrap(), list_type.file_extension());
                        self.state.write().open_list = Some(list.clone());
                    }
                }
            }
        });
    }
}
