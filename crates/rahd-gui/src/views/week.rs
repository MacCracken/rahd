//! Week view — 7-column layout with hourly grid.

use chrono::{Duration, Local, Timelike};
use egui::{Rect, Ui, Vec2};

use crate::app::RahdApp;
use crate::theme;

pub fn week_view(ui: &mut Ui, app: &RahdApp) {
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
                egui::RichText::new(day_name)
                    .strong()
                    .color(theme::ACCENT)
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

                // Column separators and current-hour highlight
                for col in 0..7 {
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

                    // Events
                    for event in app.events_on(date) {
                        let event_hour = event.start.with_timezone(&chrono::Local).hour();
                        if event_hour == hour {
                            let event_rect = Rect::from_min_size(
                                egui::pos2(col_x + 2.0, rect.top() + 2.0),
                                Vec2::new(col_width - 4.0, hour_height - 4.0),
                            );
                            painter.rect_filled(event_rect, 3.0, theme::ACCENT_DIM);
                            painter.text(
                                event_rect.left_top() + Vec2::new(3.0, 2.0),
                                egui::Align2::LEFT_TOP,
                                &event.title,
                                egui::FontId::proportional(10.0),
                                theme::TEXT_PRIMARY,
                            );
                            let time_label = event
                                .start
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
                        }
                    }
                }
            }
        });
}
