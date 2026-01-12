mod dialogs;
mod panels;
mod view_state;
mod windows;

use panels::{MainPanel, SidebarPanel, TopPanel};
pub use view_state::ViewState;
use windows::{
    CreateListWindow, ImportWindow, LegacyImportSettingsWindow, LegacyWindow, ListSettingsWindow,
    SearchWindow, SettingsWindow,
};

use crate::app::{AppRuntime, AppState, Effect};
use eframe::egui;

pub struct App {
    state: AppState,
    view_state: ViewState,
    runtime: AppRuntime,
    _tokio_runtime: tokio::runtime::Runtime,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let rt_handle = runtime.handle().clone();
        let (app_runtime, event_rx) = AppRuntime::new(rt_handle);
        let (state, init_effects) = AppState::new(event_rx);
        app_runtime.enqueue_all(init_effects);

        Self {
            state,
            view_state: ViewState::default(),
            runtime: app_runtime,
            _tokio_runtime: runtime,
        }
    }

    fn run_effects(&self, effects: Vec<Effect>) {
        self.runtime.enqueue_all(effects);
    }

    fn icon_service(&mut self, ctx: &egui::Context) {
        self.runtime.icon_service.update(ctx);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let effects = self.state.process_events();
        self.run_effects(effects);
        self.icon_service(ctx);

        if self.state.initial_loading {
            ctx.request_repaint();
            self.show_loading_screen(ctx);
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.view_state.close_all_windows();
        }

        self.render_main_ui(ctx);

        self.render_windows(ctx);
    }
}

impl App {
    fn show_loading_screen(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(240));

            ui.vertical_centered(|ui| {
                ui.add_space(rect.height() * 0.40);
                ui.heading(egui::RichText::new("Loading...").size(32.0));
                ui.add_space(20.0);
                ui.add(egui::Spinner::new().size(50.0));
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("Loading lists and metadata...")
                        .size(16.0)
                        .weak(),
                );
            });
        });
    }

    fn render_main_ui(&mut self, ctx: &egui::Context) {
        let top_effects = TopPanel::show(ctx, &mut self.view_state, &mut self.runtime);
        self.run_effects(top_effects);

        let sidebar_effects = SidebarPanel::show(
            ctx,
            &mut self.state,
            &mut self.view_state,
            &mut self.runtime,
        );
        self.run_effects(sidebar_effects);

        let main_effects = MainPanel::show(
            ctx,
            &mut self.state,
            &mut self.view_state,
            &mut self.runtime,
        );
        self.run_effects(main_effects);
    }

    fn render_windows(&mut self, ctx: &egui::Context) {
        let mut effects = Vec::new();

        if self.view_state.settings_window_open {
            let window_effects = SettingsWindow::show(ctx, &mut self.state, &mut self.view_state);
            effects.extend(window_effects);
        }

        if self.view_state.list_settings_open {
            let window_effects = ListSettingsWindow::show(
                ctx,
                &mut self.state,
                &mut self.view_state,
                &mut self.runtime,
            );
            effects.extend(window_effects);
        }

        if self.view_state.create_list_window_open {
            let window_effects = CreateListWindow::show(
                ctx,
                &mut self.state,
                &mut self.view_state,
                &mut self.runtime,
            );
            effects.extend(window_effects);
        }

        if self.view_state.import_window_open {
            let window_effects = ImportWindow::show(ctx, &mut self.state, &mut self.view_state);
            effects.extend(window_effects);
        }

        if self.view_state.search_window_open {
            let window_effects = SearchWindow::show(
                ctx,
                &mut self.state,
                &mut self.view_state,
                &mut self.runtime,
            );
            effects.extend(window_effects);
        }

        if self.state.legacy_state != crate::app::LegacyState::Idle {
            let window_effects = LegacyWindow::show(ctx, &mut self.state, &mut self.view_state);
            effects.extend(window_effects);
        }

        if self.view_state.legacy_import_settings_open {
            let window_effects =
                LegacyImportSettingsWindow::show(ctx, &mut self.state, &mut self.view_state);
            effects.extend(window_effects);
        }

        self.run_effects(effects);
    }
}
