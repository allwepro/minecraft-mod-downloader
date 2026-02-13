use egui::{Color32, CornerRadius, Frame, Stroke, TextEdit, Ui, Vec2};
use std::sync::LazyLock;

static ASH_BG: LazyLock<Color32> =
    LazyLock::new(|| Color32::from_rgba_unmultiplied(15, 15, 20, 120));
static ASH_STROKE: LazyLock<Stroke> =
    LazyLock::new(|| Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 15)));
static ASH_ROUNDING: CornerRadius = CornerRadius::same(2);

#[allow(dead_code)]
pub trait AshUi {
    fn ash<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_lite<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_frame<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_text_edit(&mut self, text: &mut String, hint: &str) -> egui::Response;
}

impl AshUi for Ui {
    fn ash<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.scope(|ui| {
            // text fíeld
            ui.style_mut().visuals.extreme_bg_color = *ASH_BG;

            //text
            ui.style_mut().visuals.override_text_color =
                Some(Color32::from_rgba_unmultiplied(220, 220, 230, 230));

            //buttons
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = *ASH_BG;
            ui.style_mut().visuals.widgets.inactive.bg_fill = *ASH_BG;
            ui.style_mut().visuals.widgets.inactive.bg_stroke = *ASH_STROKE;
            ui.style_mut().visuals.widgets.inactive.corner_radius = ASH_ROUNDING;

            ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                Color32::from_rgba_unmultiplied(40, 45, 60, 160);
            ui.style_mut().visuals.widgets.hovered.bg_fill =
                Color32::from_rgba_unmultiplied(40, 45, 60, 140);
            ui.style_mut().visuals.widgets.hovered.bg_stroke =
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
            ui.style_mut().visuals.widgets.hovered.corner_radius = ASH_ROUNDING;

            ui.style_mut().visuals.widgets.active.weak_bg_fill =
                Color32::from_rgba_unmultiplied(60, 70, 90, 180);
            ui.style_mut().visuals.widgets.active.bg_fill =
                Color32::from_rgba_unmultiplied(30, 35, 50, 160);
            ui.style_mut().visuals.widgets.active.bg_stroke =
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(150, 200, 255, 80));
            ui.style_mut().visuals.widgets.active.corner_radius = ASH_ROUNDING;

            ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                Color32::from_rgba_unmultiplied(30, 35, 50, 160);
            ui.style_mut().visuals.widgets.noninteractive.bg_stroke =
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(150, 200, 255, 80));
            ui.style_mut().visuals.widgets.noninteractive.corner_radius = ASH_ROUNDING;

            //selection
            ui.style_mut().visuals.selection.bg_fill =
                Color32::from_rgba_unmultiplied(200, 220, 255, 50);
            ui.style_mut().visuals.selection.stroke =
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 150, 255, 40));

            //spacings
            ui.style_mut().spacing.item_spacing = Vec2::splat(10.0);
            ui.style_mut().spacing.button_padding = egui::vec2(16.0, 8.0);

            add_contents(ui)
        })
        .inner
    }
    fn ash_lite<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.scope(|ui| {
            // text fíeld
            ui.style_mut().visuals.extreme_bg_color = *ASH_BG;

            //text
            ui.style_mut().visuals.override_text_color =
                Some(Color32::from_rgba_unmultiplied(220, 220, 230, 230));

            //buttons
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = *ASH_BG;
            ui.style_mut().visuals.widgets.inactive.bg_fill = *ASH_BG;
            ui.style_mut().visuals.widgets.inactive.bg_stroke = *ASH_STROKE;
            ui.style_mut().visuals.widgets.inactive.corner_radius = ASH_ROUNDING;

            ui.style_mut().visuals.widgets.hovered.weak_bg_fill =
                Color32::from_rgba_unmultiplied(40, 45, 60, 160);
            ui.style_mut().visuals.widgets.hovered.bg_fill =
                Color32::from_rgba_unmultiplied(40, 45, 60, 140);
            ui.style_mut().visuals.widgets.hovered.bg_stroke =
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
            ui.style_mut().visuals.widgets.hovered.corner_radius = ASH_ROUNDING;

            ui.style_mut().visuals.widgets.active.weak_bg_fill =
                Color32::from_rgba_unmultiplied(60, 70, 90, 180);
            ui.style_mut().visuals.widgets.active.bg_fill =
                Color32::from_rgba_unmultiplied(30, 35, 50, 160);
            ui.style_mut().visuals.widgets.active.bg_stroke =
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(150, 200, 255, 80));
            ui.style_mut().visuals.widgets.active.corner_radius = ASH_ROUNDING;

            ui.style_mut().visuals.widgets.noninteractive.bg_fill =
                Color32::from_rgba_unmultiplied(30, 35, 50, 160);
            ui.style_mut().visuals.widgets.noninteractive.bg_stroke =
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(150, 200, 255, 80));
            ui.style_mut().visuals.widgets.noninteractive.corner_radius = ASH_ROUNDING;

            //selection
            ui.style_mut().visuals.selection.bg_fill =
                Color32::from_rgba_unmultiplied(200, 220, 255, 50);
            ui.style_mut().visuals.selection.stroke =
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 150, 255, 40));

            add_contents(ui)
        })
        .inner
    }

    fn ash_frame<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        Frame::NONE
            .fill(*ASH_BG)
            .stroke(*ASH_STROKE)
            .corner_radius(ASH_ROUNDING)
            .inner_margin(12.0)
            .show(self, |ui| add_contents(ui))
            .inner
    }

    fn ash_text_edit(&mut self, text: &mut String, hint: &str) -> egui::Response {
        Frame::NONE
            .fill(*ASH_BG)
            .stroke(*ASH_STROKE)
            .corner_radius(ASH_ROUNDING)
            .inner_margin(Vec2::splat(10.0))
            .show(self, |ui| {
                ui.add(
                    TextEdit::singleline(text)
                        .frame(false)
                        .desired_width(ui.available_width())
                        .hint_text(hint),
                )
            })
            .inner
    }
}
