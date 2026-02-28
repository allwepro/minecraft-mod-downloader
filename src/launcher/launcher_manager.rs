use crate::common::ui::structs::view_controller::ViewController;
use crate::common::ui::top_panel::TopBarAction;
use crate::launcher::ui::LauncherPanel;
use crate::resource_downloader::rm_api::SharedRMAPI;
use egui::{Context, Ui};

pub struct LauncherManager {
    runtime_handle: tokio::runtime::Handle,
    rmapi: SharedRMAPI,
    launcher_panel: LauncherPanel,
}

impl LauncherManager {
    pub fn new(runtime_handle: tokio::runtime::Handle, rmapi: SharedRMAPI) -> Self {
        Self {
            runtime_handle,
            rmapi,
            launcher_panel: LauncherPanel::new(),
        }
    }
}

impl ViewController for LauncherManager {
    fn is_loaded(&self) -> bool {
        true
    }

    fn update_state(&mut self, _ctx: &Context) {
        // No-op
    }

    fn sync_frame(&mut self, _ctx: &Context) {
        // No-op
    }

    fn get_top_bar_actions(&mut self) -> Vec<TopBarAction> {
        vec![]
    }

    fn render_sidebar(&mut self, _ctx: &Context, _ui: &mut Ui) {
        // No-op
    }

    fn render_main_ui(&mut self, ctx: &Context, _ui: &mut Ui) {
        let rma = self.rmapi.read();
        let lists = rma.all_lists();
        let current_id = rma.current_list_id();
        let (download_dir, loader) = current_id
            .as_ref()
            .and_then(|id| lists.iter().find(|l| &l.id == id))
            .map(|l| (l.download_dir.clone(), l.loader.clone()))
            .unwrap_or_default();
        drop(rma);

        self.launcher_panel.show(
            ctx,
            &lists,
            &self.runtime_handle,
            &current_id,
            &download_dir,
            &loader,
        );
    }

    fn on_exit(&mut self, _tab_switch: bool, _focus_loss: bool, _exit: bool) {
        // No-op
    }
}
