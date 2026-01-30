use crate::common::prefabs::modal_window::ModalWindow;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{ListLnk, ProjectLnk};
use egui::{Color32, Id, Ui};

#[derive(Clone)]
enum LegacyModalMode {
    Progress {
        is_import: bool,
        current: usize,
        total: usize,
        message: String,
    },
    ImportResults {
        list_lnk: ListLnk,
        unresolved: Vec<String>,
    },
    ExportResults {
        unresolved: Vec<ProjectLnk>,
    },
}

#[derive(Clone)]
pub struct LegacyProgressImportModal {
    state: SharedRDState,
    mode: LegacyModalMode,
    confirmed: bool,
}

impl LegacyProgressImportModal {
    pub fn new_progress(
        is_import: bool,
        state: SharedRDState,
        current: usize,
        total: usize,
        message: String,
    ) -> Self {
        Self {
            state,
            mode: LegacyModalMode::Progress {
                is_import,
                current,
                total,
                message,
            },
            confirmed: false,
        }
    }

    pub fn new_import(state: SharedRDState, list_lnk: ListLnk, unresolved: Vec<String>) -> Self {
        Self {
            state,
            mode: LegacyModalMode::ImportResults {
                list_lnk,
                unresolved,
            },
            confirmed: false,
        }
    }

    pub fn new_export(state: SharedRDState, unresolved: Vec<ProjectLnk>) -> Self {
        Self {
            state,
            mode: LegacyModalMode::ExportResults { unresolved },
            confirmed: true,
        }
    }
}

impl ModalWindow for LegacyProgressImportModal {
    fn id(&self) -> Id {
        Id::new("legacy_operation_modal")
    }

    fn title(&self) -> String {
        match &self.mode {
            LegacyModalMode::Progress { is_import, .. } => {
                if *is_import {
                    "Importing...".into()
                } else {
                    "Exporting...".into()
                }
            }
            LegacyModalMode::ImportResults { .. } => "Import Complete".into(),
            LegacyModalMode::ExportResults { .. } => "Export Complete".into(),
        }
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        ui.set_min_width(350.0);

        match &self.mode {
            LegacyModalMode::Progress {
                current,
                total,
                message,
                ..
            } => {
                let progress = if *total > 0 {
                    *current as f32 / *total as f32
                } else {
                    0.0
                };
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.add(egui::Spinner::new().size(32.0));
                    ui.label(message);
                    ui.add(egui::ProgressBar::new(progress).text(format!("{current}/{total}")));
                });
            }

            LegacyModalMode::ImportResults { unresolved, .. } => {
                ui.vertical(|ui| {
                    ui.heading("Import Complete");

                    if !unresolved.is_empty() {
                        ui.add_space(8.0);
                        ui.colored_label(
                            Color32::GOLD,
                            format!("⚠️ {} items could not be matched.", unresolved.len()),
                        );
                    }

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        if ui.button("Import").clicked() {
                            self.confirmed = true;
                            *open = false;
                        }
                    });
                });
            }

            LegacyModalMode::ExportResults { unresolved } => {
                ui.vertical(|ui| {
                    ui.heading("Export Complete");
                    if unresolved.is_empty() {
                        ui.label("Everything exported successfully.");
                    } else {
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            format!("❌ {} items failed.", unresolved.len()),
                        );
                    }
                    ui.add_space(15.0);
                    if ui.button("Close").clicked() {
                        *open = false;
                    }
                });
            }
        }
    }

    fn on_close(&mut self) {
        if let LegacyModalMode::ImportResults { list_lnk, .. } = &self.mode {
            if !self.confirmed {
                self.state.read().list_pool.delete(list_lnk);
            }
        }
    }
}
