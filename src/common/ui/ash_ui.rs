use egui::{Color32, CornerRadius, Frame, Stroke, Style, TextEdit, Ui, Vec2};
use std::sync::LazyLock;

static ASH_BG: LazyLock<Color32> =
    LazyLock::new(|| Color32::from_rgba_unmultiplied(15, 15, 20, 120));
static ASH_STROKE: LazyLock<Stroke> =
    LazyLock::new(|| Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 15)));
static ASH_SELECT_STROKE: LazyLock<Stroke> =
    LazyLock::new(|| Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 150, 255, 15)));
pub static ASH_ROUNDING: CornerRadius = CornerRadius::same(2);

#[allow(dead_code)]
pub trait AshUi {
    // scopes
    fn ash<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_vert<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_lite<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_context_menu<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;

    // components
    fn ash_frame<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R;
    fn ash_selectable_frame(&mut self, is_selected: bool) -> Frame;
    fn ash_text_edit(&mut self, text: &mut String, hint: &str) -> egui::Response;
    fn ash_vert_text_edit(&mut self, text: &mut String, hint: &str, width: f32) -> egui::Response;
    fn ash_expand_btn(&mut self, text: String) -> egui::Response;
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

    fn ash_context_menu<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.scope(|ui| {
            apply_styles(ui.style_mut());

            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.active.weak_bg_fill = Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;

            ui.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
            ui.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
            ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;

            ui.style_mut().visuals.widgets.inactive.corner_radius = ASH_ROUNDING;
            ui.style_mut().visuals.widgets.active.corner_radius = ASH_ROUNDING;
            ui.style_mut().visuals.widgets.hovered.corner_radius = ASH_ROUNDING;
            ui.style_mut().visuals.widgets.noninteractive.corner_radius = ASH_ROUNDING;

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

    fn ash_selectable_frame(&mut self, is_selected: bool) -> Frame {
        Frame::NONE
            .fill(if is_selected {
                Color32::LIGHT_BLUE.gamma_multiply(0.1)
            } else {
                Color32::TRANSPARENT
            })
            .stroke(if is_selected {
                *ASH_SELECT_STROKE
            } else {
                *ASH_STROKE
            })
            .corner_radius(ASH_ROUNDING)
            .inner_margin(12.0)
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

    fn ash_expand_btn(&mut self, text: String) -> egui::Response {
        self.add(
            egui::Button::new(text)
                .frame(false)
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .corner_radius(ASH_ROUNDING),
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
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(200, 220, 255, 200);
    style.visuals.selection.stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(10, 50, 155, 200));
}
