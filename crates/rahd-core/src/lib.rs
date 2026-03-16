//! Rahd Core — types for calendar events, contacts, and scheduling
//!
//! Ruznam Ahd: Persian "daily record" + Arabic "appointment"

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A calendar event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub recurrence: Option<Recurrence>,
    pub reminders: Vec<Reminder>,
    pub calendar_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} - {})",
            self.title,
            self.start.format("%Y-%m-%d %H:%M"),
            self.end.format("%H:%M")
        )
    }
}

/// Recurrence rule for repeating events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Recurrence {
    Daily,
    Weekly { days: Vec<Weekday> },
    Monthly { day: u32 },
    Yearly,
}

impl fmt::Display for Recurrence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Recurrence::Daily => write!(f, "Daily"),
            Recurrence::Weekly { days } => {
                let day_strs: Vec<String> = days.iter().map(|d| d.to_string()).collect();
                write!(f, "Weekly on {}", day_strs.join(", "))
            }
            Recurrence::Monthly { day } => write!(f, "Monthly on day {day}"),
            Recurrence::Yearly => write!(f, "Yearly"),
        }
    }
}

/// Days of the week.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Weekday::Mon => write!(f, "Monday"),
            Weekday::Tue => write!(f, "Tuesday"),
            Weekday::Wed => write!(f, "Wednesday"),
            Weekday::Thu => write!(f, "Thursday"),
            Weekday::Fri => write!(f, "Friday"),
            Weekday::Sat => write!(f, "Saturday"),
            Weekday::Sun => write!(f, "Sunday"),
        }
    }
}

/// A reminder for an event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reminder {
    pub minutes_before: u32,
    pub method: ReminderMethod,
}

/// How to deliver a reminder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReminderMethod {
    Notification,
    Email,
    Sms,
}

/// A contact entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A named calendar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub is_default: bool,
}

/// A time slot (start to end).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSlot {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeSlot {
    /// Check if this slot overlaps with another.
    pub fn overlaps(&self, other: &TimeSlot) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Duration in minutes.
    pub fn duration_mins(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }
}

/// A scheduling conflict between two events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub event_a: Uuid,
    pub event_b: Uuid,
    pub overlap: TimeSlot,
}

/// Filter for listing events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub calendar_id: Option<String>,
    pub search: Option<String>,
}

impl EventFilter {
    /// Check if an event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(from) = &self.from
            && event.end < *from
        {
            return false;
        }
        if let Some(to) = &self.to
            && event.start > *to
        {
            return false;
        }
        if let Some(cal) = &self.calendar_id
            && event.calendar_id != *cal
        {
            return false;
        }
        if let Some(search) = &self.search {
            let lower = search.to_lowercase();
            if !event.title.to_lowercase().contains(&lower)
                && !event
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&lower))
            {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_event(title: &str, start_hour: u32, end_hour: u32) -> Event {
        let start = Utc.with_ymd_and_hms(2026, 3, 16, start_hour, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 16, end_hour, 0, 0).unwrap();
        Event {
            id: Uuid::new_v4(),
            title: title.to_string(),
            description: None,
            start,
            end,
            location: None,
            attendees: vec![],
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn event_creation() {
        let event = make_event("Test", 9, 10);
        assert_eq!(event.title, "Test");
        assert_eq!(event.calendar_id, "default");
    }

    #[test]
    fn event_display() {
        let event = make_event("Lunch", 12, 13);
        let display = format!("{event}");
        assert!(display.contains("Lunch"));
        assert!(display.contains("12:00"));
    }

    #[test]
    fn recurrence_display_daily() {
        let r = Recurrence::Daily;
        assert_eq!(format!("{r}"), "Daily");
    }

    #[test]
    fn recurrence_display_weekly() {
        let r = Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Wed],
        };
        let s = format!("{r}");
        assert!(s.contains("Monday"));
        assert!(s.contains("Wednesday"));
    }

    #[test]
    fn recurrence_display_monthly() {
        let r = Recurrence::Monthly { day: 15 };
        assert_eq!(format!("{r}"), "Monthly on day 15");
    }

    #[test]
    fn weekday_ordering() {
        let mut days = vec![Weekday::Fri, Weekday::Mon, Weekday::Wed];
        days.sort();
        assert_eq!(days, vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]);
    }

    #[test]
    fn timeslot_overlap_true() {
        let a = TimeSlot {
            start: Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, 11, 0, 0).unwrap(),
        };
        let b = TimeSlot {
            start: Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, 12, 0, 0).unwrap(),
        };
        assert!(a.overlaps(&b));
    }

    #[test]
    fn timeslot_no_overlap() {
        let a = TimeSlot {
            start: Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 0).unwrap(),
        };
        let b = TimeSlot {
            start: Utc.with_ymd_and_hms(2026, 3, 16, 10, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, 11, 0, 0).unwrap(),
        };
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn timeslot_duration() {
        let slot = TimeSlot {
            start: Utc.with_ymd_and_hms(2026, 3, 16, 9, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, 10, 30, 0).unwrap(),
        };
        assert_eq!(slot.duration_mins(), 90);
    }

    #[test]
    fn conflict_detection_basic() {
        let a = make_event("A", 9, 11);
        let b = make_event("B", 10, 12);
        let slot_a = TimeSlot {
            start: a.start,
            end: a.end,
        };
        let slot_b = TimeSlot {
            start: b.start,
            end: b.end,
        };
        assert!(slot_a.overlaps(&slot_b));
    }

    #[test]
    fn filter_matches_time_range() {
        let event = make_event("Meeting", 10, 11);
        let filter = EventFilter {
            from: Some(Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap()),
            to: Some(Utc.with_ymd_and_hms(2026, 3, 16, 23, 59, 59).unwrap()),
            ..Default::default()
        };
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_excludes_outside_range() {
        let event = make_event("Meeting", 10, 11);
        let filter = EventFilter {
            from: Some(Utc.with_ymd_and_hms(2026, 3, 17, 0, 0, 0).unwrap()),
            ..Default::default()
        };
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_search_text() {
        let event = make_event("Team Standup", 9, 10);
        let filter = EventFilter {
            search: Some("standup".to_string()),
            ..Default::default()
        };
        assert!(filter.matches(&event));
    }

    #[test]
    fn reminder_defaults() {
        let r = Reminder {
            minutes_before: 15,
            method: ReminderMethod::Notification,
        };
        assert_eq!(r.minutes_before, 15);
        assert_eq!(r.method, ReminderMethod::Notification);
    }
}
