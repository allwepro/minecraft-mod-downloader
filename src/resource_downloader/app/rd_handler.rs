use crate::common::modal_manager::SharedModalManager;
use crate::common::notification_manager::SharedNotificationManager;
use crate::common::pop_up_manager::SharedPopupManager;
use crate::common::prefabs::view_controller::ViewController;
use crate::common::top_panel::TopBarAction;
use crate::resource_downloader::app::modals::import_modal::ImportModal;
use crate::resource_downloader::app::modals::legacy_import_progress_modal::LegacyProgressImportModal;
use crate::resource_downloader::app::modals::modrinth_collection_import_modal::ModrinthCollectionImportModal;
use crate::resource_downloader::app::modals::settings_modal::SettingsModal;
use crate::resource_downloader::app::notifications::fail_notification::FailedNotification;
use crate::resource_downloader::app::panels::main_panel::MainPanel;
use crate::resource_downloader::app::panels::sidebar_panel::SidebarPanel;
use crate::resource_downloader::business::services::{ApiService, UpdateFn};
use crate::resource_downloader::business::{Effect, Event, InternalEvent, RDState, SharedRDState};
use crate::resource_downloader::infra::{
    ConfigManager, GameDetection, LegacyListService, ListFileManager, RDRuntime,
};
use eframe::egui;
use egui::{Context, Ui};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct RDHandler {
    _rt_handle: tokio::runtime::Handle,
    state: SharedRDState,
    update_fn: UpdateFn,

    modal_manager: SharedModalManager,
    _popups: SharedPopupManager,
    notification_manager: SharedNotificationManager,

    sidebar: SidebarPanel,
    main: MainPanel,
    settings_modal: SettingsModal,
}

impl RDHandler {
    pub fn new(
        rt_handle: tokio::runtime::Handle,
        modal_manager: SharedModalManager,
        popup_manager: SharedPopupManager,
        notification_manager: SharedNotificationManager,
    ) -> Self {
        // 1. Communication Channels
        let (effect_sx, effect_rx) = mpsc::channel::<Effect>(1024);
        let (event_sx, event_rx) = mpsc::channel::<InternalEvent>(1024);

        // 2. Infrastructure
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("flux-launcher");

        let game_detection = Arc::new(GameDetection::new());
        let config_manager = Arc::new(ConfigManager::new(config_dir.clone()));
        let list_manager = Arc::new(ListFileManager::new(config_manager.get_lists_dir()));

        let (api_service_raw, api_workers, cleanup_fn, update_fn) =
            ApiService::new(&rt_handle, config_manager.get_cache_dir());
        let api_service = Arc::new(api_service_raw);

        let legacy_list_manager = Arc::new(LegacyListService::new(api_service.clone()));

        // 3. Runtime and State
        let runtime_fn = RDRuntime::create(
            rt_handle.clone(),
            api_service.clone(),
            effect_rx,
            event_sx,
            game_detection,
            config_manager,
            list_manager,
            legacy_list_manager,
        );

        let state = Arc::new(RwLock::new(RDState::new(
            rt_handle.clone(),
            modal_manager.clone(),
            popup_manager.clone(),
            notification_manager.clone(),
            api_service,
            event_rx,
            effect_sx,
        )));

        // 4. Background Workers
        rt_handle.spawn(runtime_fn);

        for worker in api_workers {
            rt_handle.spawn(worker);
        }

        let cleanup_trigger = cleanup_fn;
        rt_handle.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                cleanup_trigger().await;
            }
        });

        // 5. Trigger Initial Business Logic
        state.write().initialize();

        Self {
            _rt_handle: rt_handle,
            state: state.clone(),
            update_fn,
            modal_manager: modal_manager.clone(),
            _popups: popup_manager.clone(),
            notification_manager: notification_manager.clone(),
            sidebar: SidebarPanel::new(state.clone()),
            main: MainPanel::new(state.clone()),
            settings_modal: SettingsModal::new(state.clone()),
        }
    }
}

impl ViewController for RDHandler {
    fn is_loaded(&self) -> bool {
        !self.state.read().loading
    }

    fn update_state(&mut self, ctx: &Context) {
        let events: Vec<Event> = {
            let mut state = self.state.write();
            let mut collected = Vec::new();
            while let Some(event) = state.next_event() {
                collected.push(event);
            }
            collected
        };

        if !events.is_empty() {
            ctx.request_repaint();
        }

        for event in events {
            match event {
                Event::Initialized { .. } => {
                    self.state.write().loading = false;
                }
                Event::ListImported { list, .. } => {
                    self.state
                        .read()
                        .submit_modal(Box::new(ImportModal::new(self.state.clone(), list)));
                }
                Event::LegacyListProgress {
                    import,
                    current,
                    total,
                    message,
                    ..
                } => {
                    self.state.read().submit_modal(Box::new(
                        LegacyProgressImportModal::new_progress(
                            import,
                            self.state.clone(),
                            current,
                            total,
                            message,
                        ),
                    ));
                }
                Event::LegacyListImported {
                    list, unresolved, ..
                } => {
                    self.state.read().submit_modal(Box::new(
                        LegacyProgressImportModal::new_import(self.state.clone(), list, unresolved),
                    ));
                }
                Event::LegacyListExported { unresolved, .. } => {
                    self.state.read().submit_modal(Box::new(
                        LegacyProgressImportModal::new_export(self.state.clone(), unresolved),
                    ));
                }
                Event::ModrinthCollectionImported {
                    collection_id,
                    contained_resource_ids,
                } => {
                    let modal = Box::new(ModrinthCollectionImportModal::new_finalizing(
                        self.state.clone(),
                        collection_id,
                        contained_resource_ids,
                    ));
                    self.state.read().submit_modal(modal);
                }
                Event::FailedModrinthCollectionImport { error, .. } => {
                    self.modal_manager.close_active();
                    self.notification_manager
                        .notify(Box::new(FailedNotification::new(
                            "Failed to Import Modrinth Collection",
                            error.as_str(),
                        )));
                }
                Event::FailedInitialization { error, .. }
                | Event::FailedListCreation { error, .. }
                | Event::FailedListDuplicated { error, .. }
                | Event::FailedListSave { error, .. }
                | Event::FailedListDelete { error, .. }
                | Event::FailedListImport { error, .. }
                | Event::FailedListExport { error, .. }
                | Event::FailedProjectVersionSelect { error, .. }
                | Event::FailedProjectArtifactDownload { error, .. }
                | Event::FailedProjectFileArchive { error, .. }
                | Event::FailedProjectFileUnarchive { error, .. }
                | Event::FailedArtifactDelete { error, .. }
                | Event::FailedLegacyListImport { error, .. }
                | Event::FailedLegacyListExport { error, .. } => {
                    self.notification_manager
                        .notify(Box::new(FailedNotification::new(
                            "Operation Failed",
                            format!("An error occurred: {error}").as_str(),
                        )));
                }
                _ => {}
            }
        }
    }

    fn sync_frame(&mut self, ctx: &Context) {
        (self.update_fn)(ctx);
    }

    fn get_top_bar_actions(&mut self) -> Vec<TopBarAction> {
        let vs = self.state.clone();
        let settings_modal = self.settings_modal.clone();
        vec![TopBarAction::new(
            "⚙ Settings",
            "Open the Settings",
            move |_ctx| {
                vs.read().submit_modal(Box::new(settings_modal));
            },
        )]
    }

    fn render_sidebar(&mut self, ctx: &Context, ui: &mut Ui) {
        self.sidebar.show(ctx, ui);
    }

    fn render_main_ui(&mut self, ctx: &Context, ui: &mut Ui) {
        self.main.show(ctx, ui);
    }

    fn on_exit(&mut self, _tab_switch: bool, _focus_loss: bool, _exit: bool) {
        if let Some(list) = self.state.read().open_list.clone() {
            self.state.read().list_pool.save(&list);
        }
    }
}
