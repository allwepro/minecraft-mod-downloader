use egui::{Color32, CornerRadius, Frame, Stroke, Style, TextEdit, Ui, Vec2};
use std::sync::LazyLock;

static ASH_BG: LazyLock<Color32> =
    LazyLock::new(|| Color32::from_rgba_unmultiplied(15, 15, 20, 120));
static ASH_STROKE: LazyLock<Stroke> =
    LazyLock::new(|| Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 15)));
static ASH_ROUNDING: CornerRadius = CornerRadius::same(2);

#[allow(dead_code)]
pub trait AshUi {
    // scopes
    fn ash<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_vert<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_lite<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;

    // components
    fn ash_frame<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_text_edit(&mut self, text: &mut String, hint: &str) -> egui::Response;
    fn ash_vert_text_edit(&mut self, text: &mut String, hint: &str, width: f32) -> egui::Response;
}

impl AshUi for Ui {
    fn ash<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.scope(|ui| {
            apply_styles(ui.style_mut());

            //spacings
            ui.style_mut().spacing.interact_size.y = 20.0;
            ui.style_mut().spacing.item_spacing = Vec2::splat(10.0);
            ui.style_mut().spacing.button_padding = egui::vec2(16.0, 8.0);

            add_contents(ui)
        })
        .inner
    }
    fn ash_vert<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.scope(|ui| {
            apply_styles(ui.style_mut());

            //spacings
            ui.style_mut().spacing.button_padding = egui::vec2(10.0, 4.0);

            add_contents(ui)
        })
        .inner
    }
    fn ash_lite<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.scope(|ui| {
            apply_styles(ui.style_mut());

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

    fn ash_vert_text_edit(&mut self, text: &mut String, hint: &str, width: f32) -> egui::Response {
        self.add(
            TextEdit::singleline(text)
                .desired_width(width)
                .hint_text(hint)
                .margin(egui::vec2(10.0, 4.0))
                .min_size(Vec2::ZERO),
        )
    }
}

fn apply_styles(style: &mut Style) {
    // text fíeld
    style.visuals.extreme_bg_color = *ASH_BG;

    //text
    style.visuals.override_text_color = Some(Color32::from_rgba_unmultiplied(220, 220, 230, 230));

    //buttons
    style.visuals.widgets.inactive.weak_bg_fill = *ASH_BG;
    style.visuals.widgets.inactive.bg_fill = *ASH_BG;
    style.visuals.widgets.inactive.bg_stroke = *ASH_STROKE;
    style.visuals.widgets.inactive.corner_radius = ASH_ROUNDING;

    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(40, 45, 60, 160);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(40, 45, 60, 140);
    style.visuals.widgets.hovered.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
    style.visuals.widgets.hovered.corner_radius = ASH_ROUNDING;

    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(60, 70, 90, 180);
    style.visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(30, 35, 50, 160);
    style.visuals.widgets.active.bg_stroke =
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(150, 200, 255, 80));
    style.visuals.widgets.active.corner_radius = ASH_ROUNDING;

    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgba_unmultiplied(30, 35, 50, 160);
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.5, Color32::from_rgba_unmultiplied(150, 200, 255, 80));
    style.visuals.widgets.noninteractive.corner_radius = ASH_ROUNDING;

    //selection
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(200, 220, 255, 50);
    style.visuals.selection.stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 150, 255, 40));
}
