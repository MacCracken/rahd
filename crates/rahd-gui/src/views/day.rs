//! Day view — hourly timeline with events.

use chrono::{Local, Timelike};
use egui::{Rect, Ui, Vec2};

use crate::app::RahdApp;
use crate::theme;

pub fn day_view(ui: &mut Ui, app: &RahdApp) {
    let events = app.events_on(app.selected_date);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let hour_height = 48.0;
            let label_width = 52.0;
            let today = Local::now().date_naive();
            let current_hour = if app.selected_date == today {
                Some(Local::now().hour())
            } else {
                None
            };

            for hour in 0..24u32 {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(available_width, hour_height), egui::Sense::hover());
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

                // Events in this hour
                for event in &events {
                    let event_hour = event
                        .start
                        .with_timezone(&chrono::Local)
                        .hour();
                    if event_hour == hour {
                        let end_hour = event.end.with_timezone(&chrono::Local).hour();
                        let duration_hours =
                            ((end_hour as f32) - (hour as f32)).max(1.0);
                        let event_rect = Rect::from_min_size(
                            egui::pos2(rect.left() + label_width + 4.0, rect.top() + 2.0),
                            Vec2::new(
                                available_width - label_width - 12.0,
                                (duration_hours * hour_height).min(hour_height - 4.0),
                            ),
                        );
                        painter.rect_filled(event_rect, 4.0, theme::ACCENT_DIM);
                        painter.text(
                            event_rect.left_top() + Vec2::new(6.0, 3.0),
                            egui::Align2::LEFT_TOP,
                            &event.title,
                            egui::FontId::proportional(12.0),
                            theme::TEXT_PRIMARY,
                        );
                        let time_label = format!(
                            "{} - {}",
                            event.start.with_timezone(&chrono::Local).format("%H:%M"),
                            event.end.with_timezone(&chrono::Local).format("%H:%M")
                        );
                        painter.text(
                            event_rect.left_top() + Vec2::new(6.0, 17.0),
                            egui::Align2::LEFT_TOP,
                            &time_label,
                            egui::FontId::proportional(10.0),
                            theme::TEXT_SECONDARY,
                        );
                    }
                }
            }
        });
}
