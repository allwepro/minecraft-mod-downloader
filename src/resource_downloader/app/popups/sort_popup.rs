use crate::common::ui::structs::popup_window::Popup;
use crate::get_list;
use crate::resource_downloader::business::SharedRDState;
use crate::resource_downloader::domain::{FilterMode, OrderMode, SortMode};
use egui::{Id, Ui};

#[derive(Clone)]
pub struct SortPopup {
    state: SharedRDState,
}

impl SortPopup {
    pub fn new(state: SharedRDState) -> Self {
        Self { state }
    }
}

impl Popup for SortPopup {
    fn id(&self) -> Id {
        Id::new("sort_popup")
    }

    fn render_contents(&mut self, ui: &mut Ui, open: &mut bool) {
        let open_list_lnk = self.state.read().open_list.clone();
        let Some(lnk) = open_list_lnk else {
            ui.label("No list open");
            return;
        };

        let list_arc = get_list!(self.state, &lnk);
        let mut list = list_arc.write();
        let mut changed = false;

        ui.set_min_width(160.0);
        ui.spacing_mut().item_spacing.y = 4.0;

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.label(egui::RichText::new("Sort by:").small().weak());

            let mut sort_mode = list.get_sort_settings().sort_mode;
            if ui
                .selectable_value(&mut sort_mode, SortMode::Name, "🔤  Name")
                .clicked()
            {
                list.set_sort_mode(sort_mode);
                changed = true;
                *open = false;
            }
            if ui
                .selectable_value(&mut sort_mode, SortMode::DateAdded, "📅  Date Added")
                .clicked()
            {
                list.set_sort_mode(sort_mode);
                changed = true;
                *open = false;
            }

            ui.separator();
            ui.label(egui::RichText::new("Order:").small().weak());
            let mut order_mode = list.get_sort_settings().order_mode;
            if ui
                .selectable_value(&mut order_mode, OrderMode::Ascending, "⬇  Ascending")
                .clicked()
            {
                list.set_order_mode(order_mode);
                changed = true;
                *open = false;
            }
            if ui
                .selectable_value(&mut order_mode, OrderMode::Descending, "⬆  Descending")
                .clicked()
            {
                list.set_order_mode(order_mode);
                changed = true;
                *open = false;
            }

            ui.separator();
            ui.label(egui::RichText::new("Filter:").small().weak());
            let mut filter_mode = list.get_sort_settings().filter_mode;
            if ui
                .selectable_value(&mut filter_mode, FilterMode::All, "⭕  All")
                .clicked()
            {
                list.set_filter_mode(filter_mode);
                changed = true;
                *open = false;
            }
            if ui
                .selectable_value(
                    &mut filter_mode,
                    FilterMode::CompatibleOnly,
                    "✅  Compatible",
                )
                .clicked()
            {
                list.set_filter_mode(filter_mode);
                changed = true;
                *open = false;
            }
            if ui
                .selectable_value(
                    &mut filter_mode,
                    FilterMode::IncompatibleOnly,
                    "❎  Incompatible",
                )
                .clicked()
            {
                list.set_filter_mode(filter_mode);
                changed = true;
                *open = false;
            }
            if ui
                .selectable_value(&mut filter_mode, FilterMode::MissingOnly, "❔  Missing")
                .clicked()
            {
                list.set_filter_mode(filter_mode);
                changed = true;
                *open = false;
            }
        });

        if changed {
            drop(list);
            self.state.read().list_pool.save(&lnk);
        }
    }
}
