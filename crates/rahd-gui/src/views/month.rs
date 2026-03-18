//! Month view — grid of day cells.

use chrono::{Datelike, Duration, Local, NaiveDate};
use egui::{Rect, Ui, Vec2};

use crate::app::RahdApp;
use crate::theme;

pub fn month_view(ui: &mut Ui, app: &mut RahdApp) {
    let today = Local::now().date_naive();
    let year = app.selected_date.year();
    let month = app.selected_date.month();
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let start_offset = first.weekday().num_days_from_monday() as i64;
    let grid_start = first - Duration::days(start_offset);

    let available = ui.available_size();
    let col_width = available.x / 7.0;
    let row_count = 6;
    let row_height = (available.y - 24.0) / row_count as f32;

    // Day-of-week header
    ui.horizontal(|ui| {
        for name in &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
            ui.add_sized(
                Vec2::new(col_width, 20.0),
                egui::Label::new(
                    egui::RichText::new(*name)
                        .small()
                        .color(theme::TEXT_MUTED),
                ),
            );
        }
    });

    // Day cells
    for row in 0..row_count {
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(available.x, row_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(rect);

        for col in 0..7 {
            let day_index = row * 7 + col;
            let date = grid_start + Duration::days(day_index as i64);
            let cell_x = rect.left() + col as f32 * col_width;
            let cell_rect = Rect::from_min_size(
                egui::pos2(cell_x, rect.top()),
                Vec2::new(col_width, row_height),
            );

            // Cell border
            painter.rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(0.5, theme::BG_WIDGET),
                egui::StrokeKind::Outside,
            );

            // Today highlight
            if date == today {
                painter.rect_filled(
                    cell_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(0, 188, 212, 15),
                );
            }

            // Day number
            let in_month = date.month() == month;
            let day_color = if date == today {
                theme::ACCENT
            } else if in_month {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_MUTED
            };
            painter.text(
                cell_rect.left_top() + Vec2::new(4.0, 2.0),
                egui::Align2::LEFT_TOP,
                format!("{}", date.day()),
                egui::FontId::proportional(11.0),
                day_color,
            );

            // Events (compact list)
            let events = app.events_on(date);
            let max_visible = ((row_height - 20.0) / 14.0) as usize;
            for (i, event) in events.iter().take(max_visible).enumerate() {
                let y_offset = 18.0 + i as f32 * 14.0;
                let tag_rect = Rect::from_min_size(
                    egui::pos2(cell_x + 3.0, cell_rect.top() + y_offset),
                    Vec2::new(col_width - 6.0, 12.0),
                );
                painter.rect_filled(tag_rect, 2.0, theme::ACCENT_DIM);
                painter.text(
                    tag_rect.left_top() + Vec2::new(3.0, 0.0),
                    egui::Align2::LEFT_TOP,
                    &event.title,
                    egui::FontId::proportional(9.0),
                    theme::TEXT_PRIMARY,
                );
            }
            if events.len() > max_visible {
                let remaining = events.len() - max_visible;
                let y_offset = 18.0 + max_visible as f32 * 14.0;
                painter.text(
                    egui::pos2(cell_x + 3.0, cell_rect.top() + y_offset),
                    egui::Align2::LEFT_TOP,
                    format!("+{remaining} more"),
                    egui::FontId::proportional(9.0),
                    theme::TEXT_MUTED,
                );
            }
        }
    }
}
