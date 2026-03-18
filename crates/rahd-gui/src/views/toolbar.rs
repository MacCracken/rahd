//! Top toolbar — navigation and period label.

use chrono::Datelike;
use egui::Ui;

use crate::app::{RahdApp, View};
use crate::theme;

pub fn toolbar(ui: &mut Ui, app: &mut RahdApp) {
    ui.horizontal(|ui| {
        if ui.button("<").clicked() {
            app.go_prev();
        }
        if ui.button("Today").clicked() {
            app.go_today();
        }
        if ui.button(">").clicked() {
            app.go_next();
        }

        ui.add_space(16.0);

        let label = match app.view {
            View::Day => app.selected_date.format("%A, %B %-d, %Y").to_string(),
            View::Week => {
                let (start, end) = app.visible_range();
                if start.month() == end.month() {
                    format!(
                        "{} {}-{}, {}",
                        start.format("%B"),
                        start.day(),
                        end.day(),
                        start.year()
                    )
                } else {
                    format!(
                        "{} {} - {} {}, {}",
                        start.format("%b"),
                        start.day(),
                        end.format("%b"),
                        end.day(),
                        end.year()
                    )
                }
            }
            View::Month => app.selected_date.format("%B %Y").to_string(),
        };
        ui.heading(egui::RichText::new(label).color(theme::TEXT_PRIMARY));
    });
}
