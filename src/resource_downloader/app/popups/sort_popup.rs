use crate::common::prefabs::popup_window::Popup;
use crate::resource_downloader::business::SharedRDState;
use egui::{Id, Ui};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Name,
    DateAdded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    All,
    CompatibleOnly,
    IncompatibleOnly,
    MissingOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OrderMode {
    #[default]
    Ascending,
    Descending,
}

#[derive(Clone, Debug)]
pub struct SortSettings {
    pub sort_mode: SortMode,
    pub order_mode: OrderMode,
    pub filter_mode: FilterMode,
}

impl Default for SortSettings {
    fn default() -> Self {
        Self {
            sort_mode: SortMode::Name,
            order_mode: OrderMode::Ascending,
            filter_mode: FilterMode::All,
        }
    }
}

#[derive(Clone)]
pub struct SortPopup {
    pub settings: Arc<RwLock<SortSettings>>,
}

impl SortPopup {
    pub fn new(_state: SharedRDState) -> Self {
        Self {
            settings: Arc::new(RwLock::new(SortSettings::default())),
        }
    }
}

impl Popup for SortPopup {
    fn id(&self) -> Id {
        Id::new("sort_popup")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        let mut settings = self.settings.write();
        ui.set_min_width(140.0);
        ui.label("Sort by:");
        if ui
            .selectable_value(&mut settings.sort_mode, SortMode::Name, "Name")
            .clicked()
        {
            *open = false;
        }
        if ui
            .selectable_value(&mut settings.sort_mode, SortMode::DateAdded, "Date Added")
            .clicked()
        {
            *open = false;
        }

        ui.separator();
        ui.label("Order:");
        if ui
            .selectable_value(
                &mut settings.order_mode,
                OrderMode::Ascending,
                "⬇ Ascending",
            )
            .clicked()
        {
            *open = false;
        }
        if ui
            .selectable_value(
                &mut settings.order_mode,
                OrderMode::Descending,
                "⬆ Descending",
            )
            .clicked()
        {
            *open = false;
        }

        ui.separator();
        ui.label("Filter:");
        if ui
            .selectable_value(&mut settings.filter_mode, FilterMode::All, "⭕ All")
            .clicked()
        {
            *open = false;
        }
        if ui
            .selectable_value(
                &mut settings.filter_mode,
                FilterMode::CompatibleOnly,
                "✅ Compatible",
            )
            .clicked()
        {
            *open = false;
        }
        if ui
            .selectable_value(
                &mut settings.filter_mode,
                FilterMode::IncompatibleOnly,
                "❎ Incompatible",
            )
            .clicked()
        {
            *open = false;
        }
        if ui
            .selectable_value(
                &mut settings.filter_mode,
                FilterMode::MissingOnly,
                "❔ Missing",
            )
            .clicked()
        {
            *open = false;
        }
    }
}
