//! Main application state and eframe::App implementation.

use chrono::{Datelike, Duration, Local, NaiveDate};
use rahd_core::{Event, EventFilter};
use rahd_store::EventStore;
use std::sync::{Arc, Mutex};

use crate::views;

/// Active calendar view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Day,
    Week,
    Month,
}

/// Main application state.
pub struct RahdApp {
    pub store: Arc<Mutex<EventStore>>,
    pub view: View,
    pub selected_date: NaiveDate,
    pub events: Vec<Event>,
    pub needs_refresh: bool,
}

impl RahdApp {
    pub fn new(store: Arc<Mutex<EventStore>>) -> Self {
        let today = Local::now().date_naive();
        let mut app = Self {
            store,
            view: View::Week,
            selected_date: today,
            events: Vec::new(),
            needs_refresh: true,
        };
        app.refresh_events();
        app
    }

    /// Load events for the current view's date range.
    pub fn refresh_events(&mut self) {
        let (from, to) = self.visible_range();
        let filter = EventFilter {
            from: Some(from.and_hms_opt(0, 0, 0).unwrap().and_utc()),
            to: Some(to.and_hms_opt(23, 59, 59).unwrap().and_utc()),
            ..Default::default()
        };
        if let Ok(store) = self.store.lock() {
            self.events = store.list_events(&filter).unwrap_or_default();
        }
        self.needs_refresh = false;
    }

    /// Get the visible date range for the current view.
    pub fn visible_range(&self) -> (NaiveDate, NaiveDate) {
        match self.view {
            View::Day => (self.selected_date, self.selected_date),
            View::Week => {
                let weekday = self.selected_date.weekday().num_days_from_monday();
                let monday = self.selected_date - Duration::days(weekday as i64);
                let sunday = monday + Duration::days(6);
                (monday, sunday)
            }
            View::Month => {
                let first = NaiveDate::from_ymd_opt(
                    self.selected_date.year(),
                    self.selected_date.month(),
                    1,
                )
                .unwrap();
                let last = if self.selected_date.month() == 12 {
                    NaiveDate::from_ymd_opt(self.selected_date.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(
                        self.selected_date.year(),
                        self.selected_date.month() + 1,
                        1,
                    )
                    .unwrap()
                } - Duration::days(1);
                (first, last)
            }
        }
    }

    /// Navigate to the previous period.
    pub fn go_prev(&mut self) {
        self.selected_date = match self.view {
            View::Day => self.selected_date - Duration::days(1),
            View::Week => self.selected_date - Duration::weeks(1),
            View::Month => {
                let (y, m) = if self.selected_date.month() == 1 {
                    (self.selected_date.year() - 1, 12)
                } else {
                    (self.selected_date.year(), self.selected_date.month() - 1)
                };
                NaiveDate::from_ymd_opt(y, m, 1).unwrap()
            }
        };
        self.needs_refresh = true;
    }

    /// Navigate to the next period.
    pub fn go_next(&mut self) {
        self.selected_date = match self.view {
            View::Day => self.selected_date + Duration::days(1),
            View::Week => self.selected_date + Duration::weeks(1),
            View::Month => {
                let (y, m) = if self.selected_date.month() == 12 {
                    (self.selected_date.year() + 1, 1)
                } else {
                    (self.selected_date.year(), self.selected_date.month() + 1)
                };
                NaiveDate::from_ymd_opt(y, m, 1).unwrap()
            }
        };
        self.needs_refresh = true;
    }

    /// Navigate to today.
    pub fn go_today(&mut self) {
        self.selected_date = Local::now().date_naive();
        self.needs_refresh = true;
    }

    /// Get events for a specific date.
    pub fn events_on(&self, date: NaiveDate) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| {
                let event_date = e.start.with_timezone(&chrono::Local).date_naive();
                event_date == date
            })
            .collect()
    }
}

impl eframe::App for RahdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.needs_refresh {
            self.refresh_events();
        }

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(180.0)
            .show(ctx, |ui| {
                views::sidebar::sidebar(ui, self);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            views::toolbar::toolbar(ui, self);
            ui.separator();

            match self.view {
                View::Day => views::day::day_view(ui, self),
                View::Week => views::week::week_view(ui, self),
                View::Month => views::month::month_view(ui, self),
            }
        });
    }
}
