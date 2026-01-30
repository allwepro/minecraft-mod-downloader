use crate::common::app_icon::show_app_icon;
use crate::common::modal_manager::SharedModalManager;
use crate::common::notification_manager::SharedNotificationManager;
use crate::common::pop_up_manager::SharedPopupManager;
use crate::common::prefabs::view_controller::ViewController;
use crate::common::top_panel::TopPanel;
use crate::resource_downloader::app::rd_handler::RDHandler;
use eframe::{egui, glow};

#[derive(Clone, Copy, PartialOrd, PartialEq)]
pub enum Tab {
    Launcher,
    ResourceDownloader,
}

pub struct App {
    _tokio_runtime: tokio::runtime::Runtime,
    last_focused: bool,

    loaded: bool,
    pub open_tab: Tab,
    last_open_tab: Tab,

    pub(crate) modal_manager: SharedModalManager,
    pub(crate) popup_manager: SharedPopupManager,
    pub(crate) notification_manager: SharedNotificationManager,

    pub(crate) rd_manager: RDHandler,
    // launcher_manager: LauncherManager,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let rt_handle = runtime.handle().clone();

        let modal_manager = SharedModalManager::default();
        let popup_manager = SharedPopupManager::default();
        let notification_manager = SharedNotificationManager::new();

        Self {
            _tokio_runtime: runtime,
            last_focused: true,
            loaded: false,
            open_tab: Tab::ResourceDownloader,
            last_open_tab: Tab::ResourceDownloader,
            modal_manager: modal_manager.clone(),
            popup_manager: popup_manager.clone(),
            notification_manager: notification_manager.clone(),
            rd_manager: RDHandler::new(
                rt_handle,
                modal_manager.clone(),
                popup_manager.clone(),
                notification_manager.clone(),
            ),
            // launcher_manager: LauncherManager::new(rt_handle),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Update active state
        self.rd_manager.update_state(ctx);
        // self.launcher_manager.update_state(ctx);
        let is_focused = ctx.input(|i| i.focused);
        if self.last_focused && !is_focused {
            self.rd_manager.on_exit(false, true, false);
            // self.launcher_manager.on_exit(false, false, true);
        }
        self.last_focused = is_focused;
        if self.last_open_tab != self.open_tab {
            match self.last_open_tab {
                Tab::ResourceDownloader => {
                    self.rd_manager.on_exit(true, false, false);
                }
                Tab::Launcher => {
                    // self.launcher_manager.on_exit(true, false, false);
                }
            }
        }
        self.last_open_tab = self.open_tab;
        match self.open_tab {
            Tab::ResourceDownloader => {
                self.rd_manager.sync_frame(ctx);
            }
            Tab::Launcher => {
                // self.launcher_manager.sync_frame(ctx);
            }
        }

        // 2. Global UI
        self.loaded = self.rd_manager.is_loaded(); // && self.launcher_manager.is_loaded();
        if !self.loaded {
            self.show_loading_screen(ctx);
            return;
        }

        self.popup_manager.begin_frame();
        TopPanel::show(ctx, self);

        // 3. Main Content Area
        if self.open_tab == Tab::ResourceDownloader {
            egui::SidePanel::left("sidebar_panel")
                .resizable(true)
                .width_range(200.0..=500.0)
                .default_width(270.0)
                .show(ctx, |ui| {
                    match self.open_tab {
                        Tab::ResourceDownloader => {
                            self.rd_manager.render_sidebar(ctx, ui);
                        }
                        Tab::Launcher => {
                            // self.launcher_manager.render_sidebar(ctx, ui);
                        }
                    }
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.open_tab {
            Tab::ResourceDownloader => {
                self.rd_manager.render_main_ui(ctx, ui);
            }
            Tab::Launcher => {
                ui.heading("Coming Soon™");
            }
        });

        // 4. Popups
        self.popup_manager.render_opened(ctx);

        // 5. Notifications
        self.notification_manager.render(ctx);

        // 6. Modals
        self.modal_manager.render(ctx, self.tab_to_id_string());

        // 7. End Frame
        self.popup_manager.end_frame(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&glow::Context>) {
        self.rd_manager.on_exit(false, false, true);
        // self.launcher_manager.on_exit(false, false, true);
    }
}

impl App {
    fn tab_to_id_string(&self) -> &'static str {
        match self.open_tab {
            Tab::Launcher => "_launcher",
            Tab::ResourceDownloader => "_resource_downloader",
        }
    }

    fn show_loading_screen(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(240));
            ui.vertical_centered(|ui| {
                ui.add_space(((ui.available_height() - 170.0) / 2.0).max(0.0));
                show_app_icon(ui, 100.0);
                ui.add_space(20.0);
                ui.add(egui::Spinner::new().size(50.0));
            });
        });
    }
}
