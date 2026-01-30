use crate::common::prefabs::notification_window::Notification;
use eframe::egui;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NotificationState {
    Active,
    Dismissed,
}

struct ActiveNotification {
    instance: Box<dyn Notification>,
    elapsed_secs: f32,
    uid: u64,
    state: NotificationState,
}

struct NotificationManagerInner {
    queue: Vec<ActiveNotification>,
    max_duration: f32,
    next_id: u64,
}

#[derive(Clone)]
pub struct SharedNotificationManager(Arc<RwLock<NotificationManagerInner>>);

impl SharedNotificationManager {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(NotificationManagerInner {
            queue: Vec::new(),
            max_duration: 8.0,
            next_id: 0,
        })))
    }

    pub fn notify(&self, notification: Box<dyn Notification>) {
        let mut inner = self.0.write();
        let uid = inner.next_id;
        inner.next_id += 1;
        inner.queue.push(ActiveNotification {
            instance: notification,
            elapsed_secs: 0.0,
            uid,
            state: NotificationState::Active,
        });
    }

    pub fn render(&self, ctx: &egui::Context) {
        let mut inner = self.0.write();
        if inner.queue.is_empty() {
            return;
        }

        ctx.request_repaint();
        let dt = ctx.input(|i| i.stable_dt);
        let max_duration = inner.max_duration;

        let mut active_idx_in_queue = None;
        let mut active_count = 0;

        for (i, item) in inner.queue.iter_mut().enumerate() {
            if item.state == NotificationState::Active {
                if active_idx_in_queue.is_none() {
                    active_idx_in_queue = Some(i);
                    item.elapsed_secs += dt;
                    if item.elapsed_secs >= max_duration {
                        item.state = NotificationState::Dismissed;
                        item.instance.on_close();
                    }
                }
                active_count += 1;
            }
        }

        let screen_rect = ctx.content_rect();
        let base_width = 320.0;
        let margin = 20.0;
        let step_offset = 8.0;

        let mut reset_timer = false;
        let mut clicked_idx = None;
        let mut closed_idx = None;

        struct RenderItem {
            queue_index: usize,
            uid: u64,
            slot: f32,
            opacity: f32,
            is_front: bool,
        }

        let mut render_items: Vec<RenderItem> = Vec::with_capacity(inner.queue.len());

        let mut active_counter = 0;

        for (i, item) in inner.queue.iter().enumerate() {
            let target_slot = if item.state == NotificationState::Dismissed {
                (active_count + 5) as f32
            } else {
                let slot = active_counter as f32;
                active_counter += 1;
                slot
            };

            let slot = ctx.animate_value_with_time(
                egui::Id::new("notif_slot").with(item.uid),
                target_slot,
                0.25,
            );

            let target_opacity = if item.state == NotificationState::Dismissed {
                0.0
            } else {
                1.0
            };
            let opacity = ctx.animate_value_with_time(
                egui::Id::new("notif_op").with(item.uid),
                target_opacity,
                0.2,
            );

            if item.state == NotificationState::Dismissed && opacity <= 0.01 {
                continue;
            }

            let visual_opacity = if slot <= 0.1 {
                opacity
            } else {
                opacity * (1.0 - (slot * 0.3)).max(0.2)
            };

            render_items.push(RenderItem {
                queue_index: i,
                uid: item.uid,
                slot,
                opacity: visual_opacity,
                is_front: slot < 0.1 && item.state == NotificationState::Active,
            });
        }

        render_items.sort_by(|a, b| b.slot.partial_cmp(&a.slot).unwrap());

        let visible_render_items: Vec<_> = render_items
            .into_iter()
            .filter(|item| item.slot < 3.0)
            .collect();

        for item in visible_render_items {
            let real_item = &inner.queue[item.queue_index];
            let title = real_item.instance.get_title().to_string();
            let desc = real_item.instance.get_desc().to_string();
            let btn_text = real_item.instance.button();
            let elapsed = real_item.elapsed_secs;

            let pos = egui::pos2(
                screen_rect.max.x - margin, /* - (item.slot * step_offset)*/
                screen_rect.max.y - margin - (item.slot * step_offset),
            );

            let area_id = egui::Id::new("notif_area").with(item.uid);
            let order = egui::Order::Tooltip;

            if item.is_front {
                ctx.move_to_top(egui::LayerId::new(order, area_id));
            }

            egui::Area::new(area_id)
                .order(order)
                .fixed_pos(pos)
                .pivot(egui::Align2::RIGHT_BOTTOM)
                .interactable(item.is_front)
                .show(ctx, |ui| {
                    let frame_color =
                        egui::Color32::from_rgb(30, 30, 30).linear_multiply(item.opacity);
                    let stroke_color = egui::Color32::from_gray(60).linear_multiply(item.opacity);

                    let mut frame = egui::Frame::window(ui.style())
                        .fill(frame_color)
                        .stroke(egui::Stroke::new(1.0, stroke_color))
                        .corner_radius(6.0);

                    if !item.is_front {
                        frame.shadow = egui::epaint::Shadow::NONE;
                    }

                    frame.show(ui, |ui| {
                        ui.set_width(base_width);

                        if item.is_front {
                            if ui.rect_contains_pointer(ui.max_rect()) {
                                reset_timer = true;
                            }

                            let progress = 1.0 - (elapsed / max_duration);
                            let mut line_rect = ui.cursor();
                            line_rect.set_height(2.0);
                            line_rect.set_width(ui.available_width());

                            ui.painter()
                                .rect_filled(line_rect, 0.0, egui::Color32::from_gray(50));
                            let mut progress_rect = line_rect;
                            progress_rect.set_width(line_rect.width() * progress);
                            ui.painter().rect_filled(
                                progress_rect,
                                0.0,
                                egui::Color32::from_rgb(166, 116, 53),
                            );
                            ui.add_space(8.0);
                        }

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(title)
                                        .strong()
                                        .color(egui::Color32::WHITE.linear_multiply(item.opacity)),
                                ));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if item.is_front {
                                            let close_btn = egui::Button::new(
                                                egui::RichText::new("❌")
                                                    .color(egui::Color32::GRAY),
                                            );
                                            if ui.add(close_btn).clicked() {
                                                closed_idx = Some(item.queue_index);
                                            }
                                        }
                                    },
                                );
                            });

                            ui.add(egui::Label::new(egui::RichText::new(desc).color(
                                egui::Color32::from_gray(180).linear_multiply(item.opacity),
                            )));

                            if let Some(text) = btn_text {
                                ui.add_space(4.0);
                                let btn = egui::Button::new(
                                    egui::RichText::new(text)
                                        .color(egui::Color32::WHITE.linear_multiply(item.opacity)),
                                )
                                .fill(
                                    egui::Color32::from_rgb(50, 50, 50)
                                        .linear_multiply(item.opacity),
                                );

                                if ui.add_enabled(item.is_front, btn).clicked() {
                                    clicked_idx = Some(item.queue_index);
                                }
                            }
                        });
                    });
                });
        }

        if reset_timer
            && let Some(idx) = active_idx_in_queue
                && let Some(item) = inner.queue.get_mut(idx) {
                    item.elapsed_secs = 0.0;
                }
        if let Some(idx) = clicked_idx {
            inner.queue[idx].instance.on_click();
        }
        if let Some(idx) = closed_idx {
            inner.queue[idx].state = NotificationState::Dismissed;
            inner.queue[idx].instance.on_close();
        }

        inner.queue.retain(|item| {
            let opacity = ctx.animate_value_with_time(
                egui::Id::new("notif_op").with(item.uid),
                if item.state == NotificationState::Dismissed {
                    0.0
                } else {
                    1.0
                },
                0.2,
            );
            opacity > 0.01
        });
    }
}
