use crate::resource_downloader::app::dialogs::Dialogs;
use crate::resource_downloader::app::modals::folder_import_modal::FolderImportModal;
use crate::resource_downloader::app::modals::legacy_import_modal::LegacyImportModal;
use crate::resource_downloader::app::modals::modrinth_collection_import_modal::ModrinthCollectionImportModal;
use crate::resource_downloader::app::notifications::fail_notification::FailedNotification;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::business::list_actions::ListActions;
use eframe::epaint::Color32;
use egui::Ui;

#[derive(Clone)]
pub struct ImportOptionsComponent {
    state: SharedRDState,
}

impl ImportOptionsComponent {
    pub fn new(state: SharedRDState) -> Self {
        Self { state }
    }

    pub fn render_contents(&mut self, ui: &mut Ui) -> bool {
        let mut clicked = false;
        if ui
            .add(
                egui::Button::new("📄 From File")
                    .min_size(egui::vec2(150.0, 0.0))
                    .fill(Color32::TRANSPARENT),
            )
            .on_hover_text("Import a List from a file. Supported formats: .mmd, .mods, .all-mods")
            .clicked()
        {
            clicked = true;
            if let Some(path) = Dialogs::pick_import_list_file() {
                match path.extension().and_then(|s| s.to_str()) {
                    Some("toml") | Some("mmd") => {
                        ListActions::import_list(self.state.clone(), path);
                    }
                    Some("mods") | Some("all-mods") | Some("queue-mods") => {
                        let sm = LegacyImportModal::new(self.state.clone(), path);
                        self.state.read().submit_modal(Box::new(sm));
                    }
                    _ => {
                        let sn = FailedNotification::new(
                            "Unsupported file type for import",
                            "The selected file type is not supported for import. Please select a valid Mod List file (.mmd) or a legacy mods file (.mods, .all-mods, .queue-mods).",
                        );
                        self.state.read().submit_notification(Box::new(sn));
                    }
                }
            }
        }
        if ui
            .add(
                egui::Button::new("🌐 Modrinth Collection")
                    .min_size(egui::vec2(150.0, 0.0))
                    .fill(Color32::TRANSPARENT),
            )
            .on_hover_text("Import a Modrinth Collection by providing its URL or ID")
            .clicked()
        {
            clicked = true;
            let sm = ModrinthCollectionImportModal::new(self.state.clone());
            self.state.read().submit_modal(Box::new(sm));
        }
        if ui
            .add(
                egui::Button::new("📁 From Folder")
                    .min_size(egui::vec2(150.0, 0.0))
                    .fill(Color32::TRANSPARENT),
            )
            .on_hover_text("Import a List from a folder containing resource files.")
            .clicked()
        {
            clicked = true;
            let sm = FolderImportModal::new(self.state.clone());
            self.state.read().submit_modal(Box::new(sm));
        }

        clicked
    }
}
