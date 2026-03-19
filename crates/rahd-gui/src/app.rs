//! Main application state and eframe::App implementation.

use chrono::{Datelike, Duration, Local, NaiveDate};
use rahd_core::{Event, EventFilter};
use rahd_store::EventStore;
use std::sync::{Arc, Mutex};

use crate::notifications::ReminderChecker;
use crate::views;

/// Active calendar view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Day,
    Week,
    Month,
}

/// Drag-and-drop state for event rescheduling.
#[derive(Debug, Clone)]
pub struct DragState {
    /// The event being dragged.
    pub event_id: uuid::Uuid,
    /// Original start time for cancellation reference.
    pub original_start: chrono::DateTime<chrono::Utc>,
    /// Event duration preserved during drag.
    pub duration: Duration,
}

/// Main application state.
pub struct RahdApp {
    pub store: Arc<Mutex<EventStore>>,
    pub view: View,
    pub selected_date: NaiveDate,
    pub events: Vec<Event>,
    pub needs_refresh: bool,
    pub reminder_checker: ReminderChecker,
    /// Active drag-and-drop operation.
    pub drag: Option<DragState>,
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
            reminder_checker: ReminderChecker::new(),
            drag: None,
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

    /// Reschedule an event to a new start time, preserving duration.
    pub fn reschedule_event(
        &mut self,
        event_id: uuid::Uuid,
        new_start: chrono::DateTime<chrono::Utc>,
        duration: Duration,
    ) {
        if let Ok(store) = self.store.lock()
            && let Ok(Some(mut event)) = store.get_event(event_id)
        {
            event.start = new_start;
            event.end = new_start + duration;
            event.updated_at = chrono::Utc::now();
            let _ = store.update_event(&event);
        }
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

        // Check for due reminders — fetch upcoming events independently of the view.
        {
            let now = chrono::Utc::now();
            let horizon = now + Duration::hours(24);
            let filter = EventFilter {
                from: Some(now - Duration::hours(1)),
                to: Some(horizon),
                ..Default::default()
            };
            let reminder_events = self
                .store
                .lock()
                .ok()
                .and_then(|s| s.list_events(&filter).ok())
                .unwrap_or_default();
            self.reminder_checker.check(&reminder_events);
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

        // Clear stale drag state if the pointer was released without a drop target.
        if self.drag.is_some() && ctx.input(|i| i.pointer.any_released()) {
            self.drag = None;
        }
    }
}
