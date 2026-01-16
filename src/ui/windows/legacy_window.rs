use crate::app::{AppState, Effect, LegacyState};
use crate::ui::ViewState;
use eframe::egui;

pub struct LegacyWindow;

impl LegacyWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
    ) -> Vec<Effect> {
        let effects = Vec::new();

        let overlay = egui::Area::new(egui::Id::new("legacy_overlay"))
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.content_rect();
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

            if ui
                .interact(
                    screen_rect,
                    egui::Id::new("legacy_overlay_click"),
                    egui::Sense::click(),
                )
                .clicked()
                && matches!(state.legacy_state, LegacyState::Complete { .. })
            {
                state.legacy_state = LegacyState::Idle;
            }
        });

        let mut is_open = true;
        let mut should_import = false;
        let mut suggested_name = String::new();

        let window_title = match &state.legacy_state {
            LegacyState::InProgress { .. } => "Processing...",
            LegacyState::Complete { is_import, .. } => {
                if *is_import {
                    "Import Complete"
                } else {
                    "Export Complete"
                }
            }
            _ => "Operation Complete",
        };

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.set_min_width(300.0);

                match &state.legacy_state {
                    LegacyState::InProgress {
                        current,
                        total,
                        message,
                    } => {
                        let progress = if *total > 0 {
                            *current as f32 / *total as f32
                        } else {
                            0.0
                        };

                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new().size(32.0));
                            ui.add_space(10.0);
                            ui.label(message);
                            ui.add(
                                egui::ProgressBar::new(progress).text(format!("{current}/{total}")),
                            );
                            ui.add_space(10.0);
                        });
                    }
                    LegacyState::Complete {
                        suggested_name: sug_name,
                        successful,
                        failed,
                        warnings,
                        is_import,
                    } => {
                        let success_count = successful.len();
                        let fail_count = failed.len();
                        let warn_count = warnings.len();
                        let is_importable = state.pending_legacy_mods.is_some();

                        suggested_name = sug_name.clone();

                        ui.vertical(|ui| {
                            ui.heading(if *is_import {
                                "Import Results"
                            } else {
                                "Export Results"
                            });
                            ui.label(format!("✅ Success: {success_count}"));

                            if fail_count > 0 {
                                ui.colored_label(
                                    egui::Color32::LIGHT_RED,
                                    format!("❌ Failed: {fail_count}"),
                                );
                            }
                            if warn_count > 0 {
                                ui.colored_label(
                                    egui::Color32::GOLD,
                                    format!("⚠️ Warnings: {warn_count}"),
                                );
                            }

                            if *is_import && is_importable && success_count > 0 {
                                ui.add_space(15.0);
                                ui.separator();
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("📥 Import into List").clicked() {
                                        should_import = true;
                                    }
                                });
                            }
                        });
                    }
                    _ => {}
                }
            });

        if should_import {
            if let Some(mods) = state.pending_legacy_mods.take() {
                view_state.legacy_import_mods = Some(mods);
                view_state.legacy_import_name = suggested_name;
                view_state.legacy_import_settings_open = true;
            }
            state.legacy_state = LegacyState::Idle;
        } else if !is_open {
            state.legacy_state = LegacyState::Idle;
            state.pending_legacy_mods = None;
        }

        effects
    }
}
