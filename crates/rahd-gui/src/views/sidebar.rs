//! Left sidebar — view switcher and mini calendar.

use chrono::{Datelike, Local, NaiveDate};
use egui::Ui;

use crate::app::{RahdApp, View};
use crate::theme;

pub fn sidebar(ui: &mut Ui, app: &mut RahdApp) {
    ui.vertical(|ui| {
        ui.add_space(8.0);
        ui.heading("Rahd");
        ui.add_space(12.0);

        // View switcher
        ui.label(egui::RichText::new("View").color(theme::TEXT_MUTED).small());
        if ui
            .selectable_label(app.view == View::Day, "  Day")
            .clicked()
        {
            app.view = View::Day;
            app.needs_refresh = true;
        }
        if ui
            .selectable_label(app.view == View::Week, "  Week")
            .clicked()
        {
            app.view = View::Week;
            app.needs_refresh = true;
        }
        if ui
            .selectable_label(app.view == View::Month, "  Month")
            .clicked()
        {
            app.view = View::Month;
            app.needs_refresh = true;
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Mini calendar
        mini_calendar(ui, app);
    });
}

fn mini_calendar(ui: &mut Ui, app: &mut RahdApp) {
    let today = Local::now().date_naive();
    let year = app.selected_date.year();
    let month = app.selected_date.month();

    // Month header with nav
    ui.horizontal(|ui| {
        if ui.small_button("<").clicked() {
            let (y, m) = if month == 1 {
                (year - 1, 12)
            } else {
                (year, month - 1)
            };
            app.selected_date = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
            app.needs_refresh = true;
        }
        let month_name = app.selected_date.format("%b %Y").to_string();
        ui.label(egui::RichText::new(month_name).strong());
        if ui.small_button(">").clicked() {
            let (y, m) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };
            app.selected_date = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
            app.needs_refresh = true;
        }
    });

    ui.add_space(4.0);

    // Day-of-week header
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for d in &["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
            ui.label(
                egui::RichText::new(*d)
                    .color(theme::TEXT_MUTED)
                    .small()
                    .monospace(),
            );
            ui.add_space(2.0);
        }
    });

    // Day grid
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let start_offset = first.weekday().num_days_from_monday();

    let days_in_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    }
    .signed_duration_since(first)
    .num_days() as u32;

    let mut day = 1u32;
    let mut cell = 0u32;
    while day <= days_in_month {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for col in 0..7u32 {
                if cell < start_offset || day > days_in_month {
                    ui.label(egui::RichText::new("  ").small().monospace());
                    ui.add_space(2.0);
                } else {
                    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                    let label = format!("{day:2}");
                    let text = if date == today {
                        egui::RichText::new(label)
                            .small()
                            .monospace()
                            .color(theme::ACCENT)
                            .strong()
                    } else if date == app.selected_date {
                        egui::RichText::new(label)
                            .small()
                            .monospace()
                            .color(theme::TEXT_PRIMARY)
                            .strong()
                    } else if col >= 5 {
                        egui::RichText::new(label)
                            .small()
                            .monospace()
                            .color(theme::TEXT_MUTED)
                    } else {
                        egui::RichText::new(label)
                            .small()
                            .monospace()
                            .color(theme::TEXT_SECONDARY)
                    };
                    if ui.label(text).clicked() {
                        app.selected_date = date;
                        app.needs_refresh = true;
                    }
                    ui.add_space(2.0);
                    day += 1;
                }
                cell += 1;
            }
        });
    }
}
