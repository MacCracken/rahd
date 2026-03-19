//! Desktop reminder notifications via notify-rust.
//!
//! Periodically checks upcoming events for due reminders and sends
//! desktop notifications. Tracks which reminders have already fired
//! to avoid duplicates.

use chrono::{Duration, Utc};
use rahd_core::{Event, ReminderMethod};
use std::collections::HashSet;
use uuid::Uuid;

/// Tracks fired reminders and checks for new ones.
pub struct ReminderChecker {
    /// Set of (event_id, minutes_before) pairs that have already fired.
    fired: HashSet<(Uuid, u32)>,
    /// Last check timestamp to throttle checks.
    last_check: chrono::DateTime<Utc>,
}

impl ReminderChecker {
    pub fn new() -> Self {
        Self {
            fired: HashSet::new(),
            last_check: Utc::now(),
        }
    }

    /// Check events for due reminders and fire desktop notifications.
    /// Should be called each frame; internally throttles to once per 30 seconds.
    pub fn check(&mut self, events: &[Event]) {
        let now = Utc::now();
        if now - self.last_check < Duration::seconds(30) {
            return;
        }
        self.last_check = now;

        for event in events {
            for reminder in &event.reminders {
                if reminder.method != ReminderMethod::Notification {
                    continue;
                }

                let key = (event.id, reminder.minutes_before);
                if self.fired.contains(&key) {
                    continue;
                }

                let trigger_time = event.start - Duration::minutes(reminder.minutes_before as i64);

                // Fire if we're within the window: trigger_time <= now < event.start
                if trigger_time <= now && now < event.start {
                    self.fired.insert(key);
                    send_notification(&event.title, reminder.minutes_before, &event.start);
                }
            }
        }

        // Prune fired set: remove entries for events that have already passed
        self.fired
            .retain(|(id, _)| events.iter().any(|e| e.id == *id && e.start > now));
    }
}

fn send_notification(title: &str, minutes_before: u32, start: &chrono::DateTime<Utc>) {
    let local_time = start.with_timezone(&chrono::Local);
    let body = if minutes_before == 0 {
        format!("Starting now ({})", local_time.format("%H:%M"))
    } else {
        format!(
            "In {} minute{} ({})",
            minutes_before,
            if minutes_before == 1 { "" } else { "s" },
            local_time.format("%H:%M")
        )
    };

    let _ = notify_rust::Notification::new()
        .appname("Rahd")
        .summary(title)
        .body(&body)
        .icon("calendar")
        .timeout(notify_rust::Timeout::Milliseconds(10_000))
        .show();
}
