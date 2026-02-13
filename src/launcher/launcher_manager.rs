use crate::common::ui::structs::view_controller::ViewController;
use crate::common::ui::top_panel::TopBarAction;
use crate::resource_downloader::rm_api::SharedRMAPI;
use egui::{Context, Ui};
//use crate::launcher::ui::LauncherPanel;

pub struct LauncherManager {
    _runtime_handle: tokio::runtime::Handle,
    rmapi: SharedRMAPI,
    //launcher_panel: LauncherPanel,
}

impl LauncherManager {
    pub fn new(runtime_handle: tokio::runtime::Handle, rmapi: SharedRMAPI) -> Self {
        Self {
            _runtime_handle: runtime_handle,
            rmapi,
            //launcher_panel: LauncherPanel::new(),
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

    fn render_main_ui(&mut self, _ctx: &Context, ui: &mut Ui) {
        ui.heading("Coming Soon™");
        let rma = self.rmapi.read();
        let lists = rma.all_lists();
        let id = rma.current_list_id();
        let _list = id.and_then(|i| lists.iter().find(|l| l.id == i));
        //self.launcher_panel.show(ctx, lists, &self.runtime_handle, list.map(|l| l.id), list.map(|l| l.download_dir), list.map(|l| l.loader));
    }

    fn on_exit(&mut self, _tab_switch: bool, _focus_loss: bool, _exit: bool) {
        // No-op
    }
}
