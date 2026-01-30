use crate::common::prefabs::popup_window::Popup;
use eframe::egui;
use egui::StrokeKind;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct PopupRequest {
    pub popup: Box<dyn Popup>,
    pub anchor_rect: egui::Rect,
}

struct PopupManagerInner {
    open_ids: Vec<egui::Id>,
    interaction_areas: HashMap<egui::Id, Vec<egui::Rect>>,
    body_rects: HashMap<egui::Id, egui::Rect>,
    requests: Vec<PopupRequest>,
    debug_mode: bool,
}

#[derive(Clone)]
pub struct SharedPopupManager(Arc<RwLock<PopupManagerInner>>);

impl Default for SharedPopupManager {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(PopupManagerInner {
            open_ids: Vec::new(),
            interaction_areas: HashMap::new(),
            body_rects: HashMap::new(),
            requests: Vec::new(),
            debug_mode: false,
        })))
    }
}

impl SharedPopupManager {
    pub fn register_interaction_area(&self, id: egui::Id, rect: egui::Rect) {
        self.0
            .write()
            .interaction_areas
            .entry(id)
            .or_default()
            .push(rect);
    }

    pub fn toggle(&self, id: egui::Id) {
        let mut inner = self.0.write();
        if inner.open_ids.contains(&id) {
            inner.open_ids.retain(|&i| i != id);
        } else {
            inner.open_ids.push(id);
        }
    }

    pub fn close(&self, id: egui::Id) {
        self.0.write().open_ids.retain(|&i| i != id);
    }

    pub fn begin_frame(&self) {
        let mut inner = self.0.write();
        inner.requests.clear();
        inner.interaction_areas.clear();

        let current_open_ids = inner.open_ids.clone();
        inner
            .body_rects
            .retain(|id, _| current_open_ids.contains(id));
    }

    pub fn request_show(&self, popup: Box<dyn Popup>, anchor_rect: egui::Rect) {
        let mut inner = self.0.write();
        let id = popup.id();
        if inner.open_ids.contains(&id) {
            inner.requests.push(PopupRequest { popup, anchor_rect });
            inner
                .interaction_areas
                .entry(id)
                .or_default()
                .push(anchor_rect);
        }
    }

    pub fn render_opened(&self, ctx: &egui::Context) {
        let requests = {
            let mut inner = self.0.write();
            std::mem::take(&mut inner.requests)
        };

        for mut req in requests {
            let id = req.popup.id();
            let last_rect =
                self.0
                    .read()
                    .body_rects
                    .get(&id)
                    .cloned()
                    .unwrap_or(egui::Rect::from_min_size(
                        req.anchor_rect.left_bottom(),
                        egui::vec2(150.0, 50.0),
                    ));

            let screen = ctx.content_rect();
            let margin = 6.0;
            let mut pos = req.anchor_rect.left_bottom() + egui::vec2(0.0, 4.0);

            if pos.y + last_rect.height() > screen.max.y - margin {
                pos.y = req.anchor_rect.top() - last_rect.height() - 4.0;
            }
            pos.y = pos.y.clamp(
                screen.min.y + margin,
                screen.max.y - last_rect.height() - margin,
            );
            pos.x = pos.x.clamp(
                screen.min.x + margin,
                screen.max.x - last_rect.width() - margin,
            );

            egui::Area::new(id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    let frame_res = egui::Frame::popup(ui.style()).show(ui, |ui| {
                        let mut is_open = true;
                        req.popup.render_contents(ui, &mut is_open);
                        if !is_open {
                            self.close(id);
                        }
                    });

                    let mut inner = self.0.write();
                    inner.body_rects.insert(id, frame_res.response.rect);
                });
        }
    }

    pub fn end_frame(&self, ctx: &egui::Context) {
        let mut inner = self.0.write();

        if inner.debug_mode {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("popup_debug"),
            ));

            for id in &inner.open_ids {
                if let Some(rects) = inner.interaction_areas.get(id) {
                    for rect in rects {
                        painter.rect_stroke(
                            *rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::GREEN),
                            StrokeKind::Middle,
                        );
                    }
                }
                if let Some(body) = inner.body_rects.get(id) {
                    painter.rect_filled(
                        *body,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(255, 0, 0, 20),
                    );
                    painter.rect_stroke(
                        *body,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::RED),
                        StrokeKind::Middle,
                    );
                }
            }
        }

        if ctx.input(|i| i.pointer.any_click())
            && let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                let mut keep_until_idx = None;

                for (idx, id) in inner.open_ids.iter().enumerate().rev() {
                    let is_hit = inner
                        .interaction_areas
                        .get(id)
                        .is_some_and(|rects| rects.iter().any(|r| r.contains(pos)))
                        || inner.body_rects.get(id).is_some_and(|r| r.contains(pos));

                    if is_hit {
                        keep_until_idx = Some(idx);
                        break;
                    }
                }

                if let Some(idx) = keep_until_idx {
                    inner.open_ids.truncate(idx + 1);
                } else {
                    inner.open_ids.clear();
                }
            }
    }
}
