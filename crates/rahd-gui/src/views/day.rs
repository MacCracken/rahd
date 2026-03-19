//! Day view — hourly timeline with events and drag-and-drop rescheduling.

use chrono::{Local, NaiveDate, Timelike};
use egui::{Rect, Ui, Vec2};

use crate::app::{DragState, RahdApp};
use crate::theme;

pub fn day_view(ui: &mut Ui, app: &mut RahdApp) {
    let selected_date = app.selected_date;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let hour_height = 48.0;
            let label_width = 52.0;
            let today = Local::now().date_naive();
            let current_hour = if selected_date == today {
                Some(Local::now().hour())
            } else {
                None
            };

            // Collect event info before mutating app
            let event_slots: Vec<_> = app
                .events_on(selected_date)
                .iter()
                .map(|e| {
                    let start_hour = e.start.with_timezone(&chrono::Local).hour();
                    let duration = e.end - e.start;
                    (e.id, e.title.clone(), e.start, start_hour, duration)
                })
                .collect();

            for hour in 0..24u32 {
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(available_width, hour_height),
                    egui::Sense::click_and_drag(),
                );
                let painter = ui.painter_at(rect);

                // Hour label
                let time_str = format!("{hour:02}:00");
                painter.text(
                    rect.left_top() + Vec2::new(4.0, 2.0),
                    egui::Align2::LEFT_TOP,
                    &time_str,
                    egui::FontId::monospace(11.0),
                    theme::TEXT_MUTED,
                );

                // Separator line
                let line_y = rect.top();
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + label_width, line_y),
                        egui::pos2(rect.right(), line_y),
                    ],
                    egui::Stroke::new(0.5, theme::BG_WIDGET),
                );

                // Current hour highlight
                if current_hour == Some(hour) {
                    let now_rect = Rect::from_min_size(
                        egui::pos2(rect.left() + label_width, rect.top()),
                        Vec2::new(available_width - label_width, hour_height),
                    );
                    painter.rect_filled(
                        now_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 188, 212, 12),
                    );
                }

                // Drop target highlight when dragging
                if app.drag.is_some() && response.hovered() {
                    let drop_rect = Rect::from_min_size(
                        egui::pos2(rect.left() + label_width, rect.top()),
                        Vec2::new(available_width - label_width, hour_height),
                    );
                    painter.rect_filled(
                        drop_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 188, 212, 25),
                    );
                }

                // Handle drop
                if response.drag_stopped()
                    && let Some(drag) = app.drag.take()
                {
                    let new_start = selected_date.and_hms_opt(hour, 0, 0).unwrap().and_utc();
                    app.reschedule_event(drag.event_id, new_start, drag.duration);
                }

                // Events in this hour
                for (id, title, start, start_hour_val, duration) in &event_slots {
                    if *start_hour_val == hour {
                        let duration_secs = duration.num_seconds() as f32;
                        let event_height =
                            (duration_secs / 3600.0 * hour_height).max(hour_height - 4.0);
                        let event_rect = Rect::from_min_size(
                            egui::pos2(rect.left() + label_width + 4.0, rect.top() + 2.0),
                            Vec2::new(available_width - label_width - 12.0, event_height),
                        );

                        // Highlight if being dragged
                        let is_dragging = app.drag.as_ref().is_some_and(|d| d.event_id == *id);
                        let bg_color = if is_dragging {
                            theme::ACCENT
                        } else {
                            theme::ACCENT_DIM
                        };

                        painter.rect_filled(event_rect, 4.0, bg_color);
                        painter.text(
                            event_rect.left_top() + Vec2::new(6.0, 3.0),
                            egui::Align2::LEFT_TOP,
                            title,
                            egui::FontId::proportional(12.0),
                            theme::TEXT_PRIMARY,
                        );
                        let time_label = format!(
                            "{} - {}",
                            start.with_timezone(&chrono::Local).format("%H:%M"),
                            (*start + *duration)
                                .with_timezone(&chrono::Local)
                                .format("%H:%M")
                        );
                        painter.text(
                            event_rect.left_top() + Vec2::new(6.0, 17.0),
                            egui::Align2::LEFT_TOP,
                            &time_label,
                            egui::FontId::proportional(10.0),
                            theme::TEXT_SECONDARY,
                        );

                        // Initiate drag on this event
                        let event_response = ui.interact(
                            event_rect,
                            egui::Id::new(("day_event_drag", *id)),
                            egui::Sense::drag(),
                        );
                        if event_response.drag_started() {
                            app.drag = Some(DragState {
                                event_id: *id,
                                original_start: *start,
                                duration: *duration,
                            });
                        }
                    }
                }
            }

            // Handle drop on the scroll area (using pointer position to determine hour)
            handle_day_drop(ui, app, selected_date, hour_height, label_width);
        });
}

/// If a drag ends and wasn't caught by a specific hour row, resolve by pointer position.
fn handle_day_drop(
    ui: &Ui,
    app: &mut RahdApp,
    date: NaiveDate,
    hour_height: f32,
    _label_width: f32,
) {
    if app.drag.is_some()
        && ui.input(|i| i.pointer.any_released())
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
    {
        let scroll_rect = ui.min_rect();
        let relative_y = pos.y - scroll_rect.top();
        let hour = ((relative_y / hour_height) as u32).clamp(0, 23);
        if let Some(drag) = app.drag.take() {
            let new_start = date.and_hms_opt(hour, 0, 0).unwrap().and_utc();
            app.reschedule_event(drag.event_id, new_start, drag.duration);
        }
    }
}
