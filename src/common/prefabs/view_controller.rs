use crate::common::top_panel::TopBarAction;
use egui::{Context, Ui};

pub trait ViewController {
    /// If the initial loading has completed
    fn is_loaded(&self) -> bool;

    /// Process backend events and return any immediate effects to be run
    fn update_state(&mut self, ctx: &Context);

    /// Call the models' frame-update logic (like icon loading)
    fn sync_frame(&mut self, ctx: &Context);

    /// Render the top bar buttons
    fn get_top_bar_actions(&mut self) -> Vec<TopBarAction>;

    /// Render sidebar panel
    fn render_sidebar(&mut self, ctx: &Context, ui: &mut Ui);

    /// Render central panel
    fn render_main_ui(&mut self, ctx: &Context, ui: &mut Ui);

    /// Handle logic when this view is closed/switched away from
    fn on_exit(&mut self, tab_switch: bool, focus_loss: bool, exit: bool);
}
