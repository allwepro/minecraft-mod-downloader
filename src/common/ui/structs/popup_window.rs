use eframe::egui;

pub trait Popup: 'static + Send + Sync {
    fn id(&self) -> egui::Id;
    fn render_contents(&mut self, ui: &mut egui::Ui, open: &mut bool);
}
