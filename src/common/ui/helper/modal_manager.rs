use crate::common::ui::ash_ui;
use crate::common::ui::ash_ui::AshUi;
use crate::common::ui::structs::modal_window::ModalWindow;
use eframe::egui;
use parking_lot::RwLock;
use std::sync::Arc;

struct ModalState {
    instance: Box<dyn ModalWindow>,
    was_initialized: bool,
}

struct ModalManagerInner {
    active_modal: Option<ModalState>,
}

#[derive(Clone)]
pub struct SharedModalManager(Arc<RwLock<ModalManagerInner>>);

impl Default for SharedModalManager {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(ModalManagerInner {
            active_modal: None,
        })))
    }
}

#[allow(dead_code)]
impl SharedModalManager {
    pub fn open(&self, modal: Box<dyn ModalWindow>) {
        let old = {
            let mut inner = self.0.write();
            let old = inner.active_modal.take();
            inner.active_modal = Some(ModalState {
                instance: modal,
                was_initialized: false,
            });
            old
        };

        if let Some(mut old_modal) = old {
            old_modal.instance.on_close();
        }
    }

    pub fn is_any_open(&self) -> bool {
        self.0.read().active_modal.is_some()
    }

    pub fn close_active(&self) {
        let old = {
            let mut inner = self.0.write();
            inner.active_modal.take()
        };

        if let Some(mut modal) = old {
            modal.instance.on_close();
        }
    }

    pub fn render(&self, ctx: &egui::Context, tab_str: &str) {
        let modal_state = self.0.write().active_modal.take();

        if let Some(mut state) = modal_state {
            if !state.was_initialized {
                state.instance.on_open();
                state.was_initialized = true;
            }

            let mut is_open = true;
            let mut is_open_internal = true;

            let overlay_id = state.instance.id().with("overlay").with(tab_str);
            egui::Area::new(overlay_id)
                .order(egui::Order::Background)
                .fixed_pos(egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    let screen_rect = ctx.content_rect();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_black_alpha(128),
                    );

                    if ui
                        .interact(screen_rect, overlay_id.with("click"), egui::Sense::click())
                        .clicked()
                    {
                        is_open = false;
                    }
                });

            ctx.style_mut(|s| {
                s.visuals.window_corner_radius = ash_ui::ASH_ROUNDING;
                s.visuals.widgets.open.weak_bg_fill = egui::Color32::TRANSPARENT;
            });
            egui::Window::new(state.instance.title())
                .id(state.instance.id().with(tab_str))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut is_open)
                .show(ctx, |ui| {
                    ui.ash_lite(|ui| {
                        state.instance.render_contents(ui, &mut is_open_internal);
                    });
                });

            let mut inner = self.0.write();
            let should_close =
                !is_open || !is_open_internal || ctx.input(|i| i.key_pressed(egui::Key::Escape));

            if should_close {
                drop(inner);
                state.instance.on_close();
            } else if inner.active_modal.is_none() {
                inner.active_modal = Some(state);
            } else {
                drop(inner);
                state.instance.on_close();
            }
        }
    }
}
