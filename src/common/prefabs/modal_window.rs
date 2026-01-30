use eframe::egui;

pub trait ModalWindow: 'static + Send + Sync {
    fn id(&self) -> egui::Id;
    fn title(&self) -> String;
    fn render_contents(&mut self, ui: &mut egui::Ui, open: &mut bool);

    // Lifecycle Hooks
    fn on_open(&mut self) {}
    fn on_close(&mut self) {}
}
