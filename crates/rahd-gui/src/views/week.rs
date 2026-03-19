//! Week view — 7-column layout with hourly grid and drag-and-drop rescheduling.

use chrono::{Duration, Local, Timelike};
use egui::{Rect, Ui, Vec2};

use crate::app::{DragState, RahdApp};
use crate::theme;

pub fn week_view(ui: &mut Ui, app: &mut RahdApp) {
    let (week_start, _) = app.visible_range();
    let today = Local::now().date_naive();
    let available_width = ui.available_width();
    let label_width = 52.0;
    let col_width = (available_width - label_width) / 7.0;

    // Day headers
    ui.horizontal(|ui| {
        ui.add_space(label_width);
        for i in 0..7 {
            let date = week_start + Duration::days(i);
            let day_name = date.format("%a %-d").to_string();
            let text = if date == today {
                egui::RichText::new(day_name).strong().color(theme::ACCENT)
            } else {
                egui::RichText::new(day_name).color(theme::TEXT_SECONDARY)
            };
            ui.add_sized(Vec2::new(col_width, 20.0), egui::Label::new(text));
        }
    });
    ui.separator();

    // Hourly grid
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let hour_height = 40.0;
            let current_hour = if (week_start..=week_start + Duration::days(6)).contains(&today) {
                Some(Local::now().hour())
            } else {
                None
            };

            // Pre-collect events per day-column to avoid borrow issues
            let day_events: Vec<Vec<_>> = (0..7)
                .map(|i| {
                    let date = week_start + Duration::days(i);
                    app.events_on(date)
                        .iter()
                        .map(|e| {
                            let start_hour = e.start.with_timezone(&chrono::Local).hour();
                            let duration = e.end - e.start;
                            (e.id, e.title.clone(), e.start, start_hour, duration)
                        })
                        .collect()
                })
                .collect();

            // Check if pointer was just released (for drop detection)
            let pointer_released = app.drag.is_some() && ui.input(|i| i.pointer.any_released());

            for hour in 6..22u32 {
                let row_width = available_width;
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(row_width, hour_height), egui::Sense::hover());
                let painter = ui.painter_at(rect);

                // Hour label
                let time_str = format!("{hour:02}:00");
                painter.text(
                    rect.left_top() + Vec2::new(4.0, 2.0),
                    egui::Align2::LEFT_TOP,
                    &time_str,
                    egui::FontId::monospace(10.0),
                    theme::TEXT_MUTED,
                );

                // Grid lines
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + label_width, rect.top()),
                        egui::pos2(rect.right(), rect.top()),
                    ],
                    egui::Stroke::new(0.5, theme::BG_WIDGET),
                );

                // Column separators, highlights, events, and drop targets
                for col in 0..7i64 {
                    let col_x = rect.left() + label_width + col as f32 * col_width;
                    painter.line_segment(
                        [
                            egui::pos2(col_x, rect.top()),
                            egui::pos2(col_x, rect.bottom()),
                        ],
                        egui::Stroke::new(0.5, theme::BG_WIDGET),
                    );

                    let date = week_start + Duration::days(col);
                    if date == today && current_hour == Some(hour) {
                        let highlight = Rect::from_min_size(
                            egui::pos2(col_x, rect.top()),
                            Vec2::new(col_width, hour_height),
                        );
                        painter.rect_filled(
                            highlight,
                            0.0,
                            egui::Color32::from_rgba_premultiplied(0, 188, 212, 12),
                        );
                    }

                    // Drop target cell
                    let cell_rect = Rect::from_min_size(
                        egui::pos2(col_x, rect.top()),
                        Vec2::new(col_width, hour_height),
                    );
                    let cell_response = ui.interact(
                        cell_rect,
                        egui::Id::new(("week_cell", col, hour)),
                        egui::Sense::hover(),
                    );

                    // Highlight drop target
                    if app.drag.is_some() && cell_response.hovered() {
                        painter.rect_filled(
                            cell_rect,
                            0.0,
                            egui::Color32::from_rgba_premultiplied(0, 188, 212, 25),
                        );
                    }

                    // Handle drop: pointer released while hovering this cell
                    if pointer_released
                        && cell_response.hovered()
                        && let Some(drag) = app.drag.take()
                    {
                        let new_start = date.and_hms_opt(hour, 0, 0).unwrap().and_utc();
                        app.reschedule_event(drag.event_id, new_start, drag.duration);
                    }

                    // Events
                    for (id, title, start, start_hour_val, duration) in &day_events[col as usize] {
                        if *start_hour_val == hour {
                            let event_rect = Rect::from_min_size(
                                egui::pos2(col_x + 2.0, rect.top() + 2.0),
                                Vec2::new(col_width - 4.0, hour_height - 4.0),
                            );

                            let is_dragging = app.drag.as_ref().is_some_and(|d| d.event_id == *id);
                            let bg_color = if is_dragging {
                                theme::ACCENT
                            } else {
                                theme::ACCENT_DIM
                            };

                            painter.rect_filled(event_rect, 3.0, bg_color);
                            painter.text(
                                event_rect.left_top() + Vec2::new(3.0, 2.0),
                                egui::Align2::LEFT_TOP,
                                title,
                                egui::FontId::proportional(10.0),
                                theme::TEXT_PRIMARY,
                            );
                            let time_label = start
                                .with_timezone(&chrono::Local)
                                .format("%H:%M")
                                .to_string();
                            painter.text(
                                event_rect.left_top() + Vec2::new(3.0, 14.0),
                                egui::Align2::LEFT_TOP,
                                &time_label,
                                egui::FontId::proportional(9.0),
                                theme::TEXT_SECONDARY,
                            );

                            // Drag source
                            let drag_response = ui.interact(
                                event_rect,
                                egui::Id::new(("week_event_drag", *id)),
                                egui::Sense::drag(),
                            );
                            if drag_response.drag_started() {
                                app.drag = Some(DragState {
                                    event_id: *id,
                                    original_start: *start,
                                    duration: *duration,
                                });
                            }
                        }
                    }
                }
            }
        });
}
