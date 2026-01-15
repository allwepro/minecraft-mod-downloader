use crate::app::{AppRuntime, AppState, Effect};
use crate::ui::ViewState;
use eframe::egui;

pub struct SearchWindow;

impl SearchWindow {
    pub fn show(
        ctx: &egui::Context,
        state: &mut AppState,
        view_state: &mut ViewState,
        runtime: &mut AppRuntime,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();

        if state.current_list_id.is_none() {
            view_state.search_window_open = false;
            return effects;
        }

        let overlay = egui::Area::new(egui::Id::new("search_overlay"))
            .order(egui::Order::Background)
            .fixed_pos(egui::pos2(0.0, 0.0));

        overlay.show(ctx, |ui| {
            let screen_rect = ctx.content_rect();
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

            if ui
                .interact(
                    screen_rect,
                    egui::Id::new("search_overlay_click"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                view_state.search_window_open = false;
            }
        });

        let current_type = state.get_current_list_type();
        let mut is_open = view_state.search_window_open;
        let mut mod_to_add = None;
        let mut should_close_window = false;

        egui::Window::new(format!("🔍 Search {}", current_type.display_name()))
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 400.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let query_response = ui.add(
                        egui::TextEdit::singleline(&mut view_state.search_window_query)
                            .hint_text("Search name or description...")
                            .desired_width(400.0),
                    );

                    if query_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        view_state.is_searching = true;
                        effects.extend(state.perform_search(&view_state.search_window_query));
                    }

                    ui.checkbox(&mut state.search_filter_exact, "Match version/loader");

                    if ui.button("Search").clicked() {
                        view_state.is_searching = true;
                        effects.extend(state.perform_search(&view_state.search_window_query));
                    }
                });
                ui.separator();

                if !state.search_window_results.is_empty()
                    || view_state.search_window_query.is_empty()
                {
                    view_state.is_searching = false;
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if view_state.is_searching && state.search_window_results.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new().size(48.0));
                            ui.add_space(10.0);
                            ui.label("Searching...");
                            ui.add_space(10.0);
                        });
                    } else if state.search_window_results.is_empty() {
                        ui.label("Enter a search query");
                    } else {
                        for mod_info in &state.search_window_results {
                            ui.horizontal(|ui| {
                                if !mod_info.icon_url.is_empty() {
                                    if let Some(handle) =
                                        runtime.icon_service.get(&mod_info.icon_url)
                                    {
                                        ui.add(
                                            egui::Image::from_texture(handle)
                                                .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                                        );
                                    } else {
                                        ui.add_sized(egui::vec2(32.0, 32.0), egui::Spinner::new());
                                    }
                                } else {
                                    ui.add_space(32.0);
                                }
                                ui.add_space(4.0);

                                let button_width = 50.0;
                                let spacing = 8.0;
                                let available_width = ui.available_width() - button_width - spacing;

                                ui.vertical(|ui| {
                                    ui.set_max_width(available_width);
                                    let project_link = runtime
                                        .get_project_link(&mod_info.project_type, &mod_info.id);
                                    ui.hyperlink_to(&mod_info.name, project_link);
                                    ui.add(
                                        egui::Label::new(&mod_info.description)
                                            .wrap_mode(egui::TextWrapMode::Wrap),
                                    );
                                    ui.label(format!(
                                        "👤 {} | ⬇ {}",
                                        mod_info.author, mod_info.download_count
                                    ));
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Add").clicked() {
                                            mod_to_add = Some(mod_info.clone());
                                            should_close_window = true;
                                        }
                                    },
                                );
                            });
                            ui.separator();
                        }
                    }
                });

                if let Some(mod_info) = mod_to_add {
                    effects.extend(state.add_mod_to_current_list(mod_info.clone()));
                    effects.extend(state.load_mod_details_if_needed(&mod_info.id));
                }
            });

        if should_close_window {
            view_state.search_window_open = false;
            view_state.search_window_query.clear();
            view_state.is_searching = false;
        } else {
            view_state.search_window_open = is_open;
        }

        if !view_state.search_window_open {
            view_state.search_window_query.clear();
            view_state.is_searching = false;
            state.search_window_results.clear();
        }

        effects
    }
}
