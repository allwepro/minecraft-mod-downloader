use crate::common::ui::structs::popup_window::Popup;
use crate::resource_downloader::app::components::import_options_component::ImportOptionsComponent;
use crate::resource_downloader::business::SharedRDState;
use egui::{Id, Ui};

#[derive(Clone)]
pub struct ImportPopup {
    import_options: ImportOptionsComponent,
}

impl ImportPopup {
    pub fn new(state: SharedRDState) -> Self {
        Self {
            import_options: ImportOptionsComponent::new(state),
        }
    }
}

impl Popup for ImportPopup {
    fn id(&self) -> Id {
        Id::new("import_popup")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        if self.import_options.render_contents(ui) {
            *open = false;
        }
    }
}
