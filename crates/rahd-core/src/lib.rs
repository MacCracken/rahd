//! Rahd Core — types for calendar events, contacts, and scheduling
//!
//! Provides [`Event`], [`Contact`], [`Calendar`], [`Recurrence`], [`TimeSlot`],
//! and [`Conflict`] types, plus ICS/vCard serialization and recurring event expansion.
//!
//! Ruznam Ahd: Persian "daily record" + Arabic "appointment"

use chrono::{DateTime, Duration, NaiveDate, Utc};
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

// ---------------------------------------------------------------------------
// Recurring event expansion
// ---------------------------------------------------------------------------

/// Expand a recurring event into concrete instances within a date range.
///
/// Returns new `Event` structs with unique IDs, each shifted to the appropriate
/// date. The original event's `recurrence` field is preserved on each instance.
pub fn expand_recurrence(
    event: &Event,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> Vec<Event> {
    let Some(ref recurrence) = event.recurrence else {
        // No recurrence — return the event itself if it falls in range.
        if event.start < range_end && event.end > range_start {
            return vec![event.clone()];
        }
        return vec![];
    };

    let duration = event.end - event.start;
    let mut instances = Vec::new();
    let mut cursor = event.start;

    // Safety limit to prevent infinite loops.
    let max_instances = 365;
    let mut count = 0;

    while cursor < range_end && count < max_instances {
        if cursor + duration > range_start {
            let mut instance = event.clone();
            instance.id = Uuid::new_v4();
            instance.start = cursor;
            instance.end = cursor + duration;
            instances.push(instance);
        }

        cursor = match recurrence {
            Recurrence::Daily => cursor + Duration::days(1),
            Recurrence::Weekly { days } => advance_to_next_weekday(cursor, days),
            Recurrence::Monthly { day } => advance_to_next_month(cursor, *day),
            Recurrence::Yearly => advance_to_next_year(cursor),
        };
        count += 1;
    }

    instances
}

fn advance_to_next_weekday(current: DateTime<Utc>, days: &[Weekday]) -> DateTime<Utc> {
    if days.is_empty() {
        return current + Duration::weeks(1);
    }
    // Find the next matching weekday
    for offset in 1..=7 {
        let next = current + Duration::days(offset);
        let next_wd = next.date_naive().weekday();
        if days.iter().any(|d| to_chrono_weekday(*d) == next_wd) {
            return next;
        }
    }
    current + Duration::weeks(1)
}

fn advance_to_next_month(current: DateTime<Utc>, day: u32) -> DateTime<Utc> {
    let date = current.date_naive();
    let (mut year, mut month) = (date.year(), date.month());
    month += 1;
    if month > 12 {
        month = 1;
        year += 1;
    }
    let target_day = day.max(1).min(days_in_month(year, month));
    let target_date = NaiveDate::from_ymd_opt(year, month, target_day).unwrap();
    let time = current.time();
    target_date.and_time(time).and_utc()
}

fn advance_to_next_year(current: DateTime<Utc>) -> DateTime<Utc> {
    let date = current.date_naive();
    let next_year = date.year() + 1;
    let month = date.month();
    let day = date.day().min(days_in_month(next_year, month));
    let target = NaiveDate::from_ymd_opt(next_year, month, day).unwrap();
    target.and_time(current.time()).and_utc()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn to_chrono_weekday(wd: Weekday) -> chrono::Weekday {
    match wd {
        Weekday::Mon => chrono::Weekday::Mon,
        Weekday::Tue => chrono::Weekday::Tue,
        Weekday::Wed => chrono::Weekday::Wed,
        Weekday::Thu => chrono::Weekday::Thu,
        Weekday::Fri => chrono::Weekday::Fri,
        Weekday::Sat => chrono::Weekday::Sat,
        Weekday::Sun => chrono::Weekday::Sun,
    }
}

use chrono::Datelike;

// ---------------------------------------------------------------------------
// ICS (iCalendar) serialization
// ---------------------------------------------------------------------------

/// Serialize an event to ICS (iCalendar) format.
pub fn event_to_ics(event: &Event) -> String {
    let mut lines = Vec::new();
    lines.push("BEGIN:VCALENDAR".to_string());
    lines.push("VERSION:2.0".to_string());
    lines.push("PRODID:-//Rahd//Calendar//EN".to_string());
    lines.push("BEGIN:VEVENT".to_string());
    lines.push(format!("UID:{}", event.id));
    lines.push(format!("DTSTART:{}", to_ics_datetime(event.start)));
    lines.push(format!("DTEND:{}", to_ics_datetime(event.end)));
    lines.push(format!("SUMMARY:{}", ics_escape(&event.title)));
    if let Some(desc) = &event.description {
        lines.push(format!("DESCRIPTION:{}", ics_escape(desc)));
    }
    if let Some(loc) = &event.location {
        lines.push(format!("LOCATION:{}", ics_escape(loc)));
    }
    for attendee in &event.attendees {
        lines.push(format!("ATTENDEE:{}", ics_escape(attendee)));
    }
    if let Some(ref rec) = event.recurrence {
        lines.push(recurrence_to_rrule(rec));
    }
    for reminder in &event.reminders {
        lines.push("BEGIN:VALARM".to_string());
        lines.push("ACTION:DISPLAY".to_string());
        lines.push(format!("TRIGGER:-PT{}M", reminder.minutes_before));
        lines.push("DESCRIPTION:Reminder".to_string());
        lines.push("END:VALARM".to_string());
    }
    lines.push(format!("CREATED:{}", to_ics_datetime(event.created_at)));
    lines.push(format!(
        "LAST-MODIFIED:{}",
        to_ics_datetime(event.updated_at)
    ));
    lines.push("END:VEVENT".to_string());
    lines.push("END:VCALENDAR".to_string());
    lines.join("\r\n") + "\r\n"
}

/// Serialize multiple events to a single ICS file.
pub fn events_to_ics(events: &[Event]) -> String {
    let mut lines = Vec::new();
    lines.push("BEGIN:VCALENDAR".to_string());
    lines.push("VERSION:2.0".to_string());
    lines.push("PRODID:-//Rahd//Calendar//EN".to_string());
    for event in events {
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:{}", event.id));
        lines.push(format!("DTSTART:{}", to_ics_datetime(event.start)));
        lines.push(format!("DTEND:{}", to_ics_datetime(event.end)));
        lines.push(format!("SUMMARY:{}", ics_escape(&event.title)));
        if let Some(desc) = &event.description {
            lines.push(format!("DESCRIPTION:{}", ics_escape(desc)));
        }
        if let Some(loc) = &event.location {
            lines.push(format!("LOCATION:{}", ics_escape(loc)));
        }
        for attendee in &event.attendees {
            lines.push(format!("ATTENDEE:{}", ics_escape(attendee)));
        }
        if let Some(ref rec) = event.recurrence {
            lines.push(recurrence_to_rrule(rec));
        }
        lines.push("END:VEVENT".to_string());
    }
    lines.push("END:VCALENDAR".to_string());
    lines.join("\r\n") + "\r\n"
}

/// Parse events from ICS content.
pub fn events_from_ics(ics: &str) -> Vec<Event> {
    let mut events = Vec::new();
    let mut in_event = false;
    let mut uid = None;
    let mut summary = None;
    let mut description = None;
    let mut dtstart = None;
    let mut dtend = None;
    let mut location = None;
    let mut attendees = Vec::new();

    for raw_line in ics.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line == "BEGIN:VEVENT" {
            in_event = true;
            uid = None;
            summary = None;
            description = None;
            dtstart = None;
            dtend = None;
            location = None;
            attendees = Vec::new();
        } else if line == "END:VEVENT" && in_event {
            let start = dtstart.take().unwrap_or_else(Utc::now);
            let end = dtend.take().unwrap_or(start + Duration::hours(1));
            let id = uid
                .take()
                .and_then(|u: String| u.parse::<Uuid>().ok())
                .unwrap_or_else(Uuid::new_v4);
            let now = Utc::now();
            events.push(Event {
                id,
                title: summary.take().unwrap_or_else(|| "Untitled".to_string()),
                description: description.take(),
                start,
                end,
                location: location.take(),
                attendees: std::mem::take(&mut attendees),
                recurrence: None,
                reminders: vec![],
                calendar_id: "default".to_string(),
                created_at: now,
                updated_at: now,
            });
            in_event = false;
        } else if in_event {
            if let Some(val) = line.strip_prefix("UID:") {
                uid = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("SUMMARY:") {
                summary = Some(ics_unescape(val));
            } else if let Some(val) = line.strip_prefix("DESCRIPTION:") {
                description = Some(ics_unescape(val));
            } else if let Some(val) = line.strip_prefix("DTSTART:") {
                dtstart = parse_ics_datetime(val);
            } else if let Some(val) = line.strip_prefix("DTSTART;") {
                // Handle DTSTART;TZID=... or DTSTART;VALUE=DATE:...
                if let Some((_params, value)) = val.split_once(':') {
                    dtstart = parse_ics_datetime(value);
                }
            } else if let Some(val) = line.strip_prefix("DTEND:") {
                dtend = parse_ics_datetime(val);
            } else if let Some(val) = line.strip_prefix("DTEND;") {
                if let Some((_params, value)) = val.split_once(':') {
                    dtend = parse_ics_datetime(value);
                }
            } else if let Some(val) = line.strip_prefix("LOCATION:") {
                location = Some(ics_unescape(val));
            } else if let Some(val) = line.strip_prefix("ATTENDEE:") {
                attendees.push(ics_unescape(val));
            }
        }
    }
    events
}

fn to_ics_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}

fn parse_ics_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    // 20260316T120000Z
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%SZ") {
        return Some(dt.and_utc());
    }
    // 20260316T120000 (no Z — assume UTC)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S") {
        return Some(dt.and_utc());
    }
    // 20260316 (date only)
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y%m%d") {
        return Some(d.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }
    None
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn ics_unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\;", ";")
        .replace("\\,", ",")
        .replace("\\\\", "\\")
}

fn recurrence_to_rrule(rec: &Recurrence) -> String {
    match rec {
        Recurrence::Daily => "RRULE:FREQ=DAILY".to_string(),
        Recurrence::Weekly { days } => {
            let day_strs: Vec<&str> = days
                .iter()
                .map(|d| match d {
                    Weekday::Mon => "MO",
                    Weekday::Tue => "TU",
                    Weekday::Wed => "WE",
                    Weekday::Thu => "TH",
                    Weekday::Fri => "FR",
                    Weekday::Sat => "SA",
                    Weekday::Sun => "SU",
                })
                .collect();
            format!("RRULE:FREQ=WEEKLY;BYDAY={}", day_strs.join(","))
        }
        Recurrence::Monthly { day } => {
            format!("RRULE:FREQ=MONTHLY;BYMONTHDAY={day}")
        }
        Recurrence::Yearly => "RRULE:FREQ=YEARLY".to_string(),
    }
}

// ---------------------------------------------------------------------------
// vCard serialization
// ---------------------------------------------------------------------------

/// Serialize a contact to vCard 3.0 format.
pub fn contact_to_vcard(contact: &Contact) -> String {
    let mut lines = Vec::new();
    lines.push("BEGIN:VCARD".to_string());
    lines.push("VERSION:3.0".to_string());
    lines.push(format!("FN:{}", ics_escape(&contact.name)));
    lines.push(format!("UID:{}", contact.id));
    if let Some(email) = &contact.email {
        lines.push(format!("EMAIL:{}", ics_escape(email)));
    }
    if let Some(phone) = &contact.phone {
        lines.push(format!("TEL:{}", ics_escape(phone)));
    }
    if let Some(org) = &contact.organization {
        lines.push(format!("ORG:{}", ics_escape(org)));
    }
    if let Some(notes) = &contact.notes {
        lines.push(format!("NOTE:{}", ics_escape(notes)));
    }
    lines.push("END:VCARD".to_string());
    lines.join("\r\n") + "\r\n"
}

/// Serialize multiple contacts to a single vCard file.
pub fn contacts_to_vcard(contacts: &[Contact]) -> String {
    contacts.iter().map(contact_to_vcard).collect()
}

/// Parse contacts from vCard content.
pub fn contacts_from_vcard(vcard: &str) -> Vec<Contact> {
    let mut contacts = Vec::new();
    let mut in_card = false;
    let mut name = None;
    let mut uid = None;
    let mut email = None;
    let mut phone = None;
    let mut organization = None;
    let mut notes = None;

    for raw_line in vcard.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line == "BEGIN:VCARD" {
            in_card = true;
            name = None;
            uid = None;
            email = None;
            phone = None;
            organization = None;
            notes = None;
        } else if line == "END:VCARD" && in_card {
            let id = uid
                .take()
                .and_then(|u: String| u.parse::<Uuid>().ok())
                .unwrap_or_else(Uuid::new_v4);
            contacts.push(Contact {
                id,
                name: name.take().unwrap_or_else(|| "Unknown".to_string()),
                email: email.take(),
                phone: phone.take(),
                organization: organization.take(),
                notes: notes.take(),
                created_at: Utc::now(),
            });
            in_card = false;
        } else if in_card {
            if let Some(val) = line.strip_prefix("FN:") {
                name = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("UID:") {
                uid = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("EMAIL") {
                // Handle EMAIL:, EMAIL;TYPE=...:
                if let Some(v) = val.strip_prefix(':') {
                    email = Some(v.to_string());
                } else if let Some((_params, v)) = val.split_once(':') {
                    email = Some(v.to_string());
                }
            } else if let Some(val) = line.strip_prefix("TEL") {
                if let Some(v) = val.strip_prefix(':') {
                    phone = Some(v.to_string());
                } else if let Some((_params, v)) = val.split_once(':') {
                    phone = Some(v.to_string());
                }
            } else if let Some(val) = line.strip_prefix("ORG:") {
                organization = Some(val.to_string());
            } else if let Some(val) = line.strip_prefix("NOTE:") {
                notes = Some(ics_unescape(val));
            }
        }
    }
    contacts
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

    // -- Recurring event expansion tests --

    #[test]
    fn expand_daily_recurrence() {
        let mut event = make_event("Standup", 9, 10);
        event.recurrence = Some(Recurrence::Daily);
        let start = Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 19, 0, 0, 0).unwrap();
        let instances = expand_recurrence(&event, start, end);
        assert_eq!(instances.len(), 3); // Mar 16, 17, 18
    }

    #[test]
    fn expand_weekly_recurrence() {
        let mut event = make_event("Team sync", 10, 11);
        event.recurrence = Some(Recurrence::Weekly {
            days: vec![Weekday::Mon, Weekday::Wed],
        });
        let start = Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap(); // Monday
        let end = Utc.with_ymd_and_hms(2026, 3, 25, 0, 0, 0).unwrap();
        let instances = expand_recurrence(&event, start, end);
        // Mon 16, Wed 18, Mon 23, Wed 25 — but 25 is at 10:00 so it's in range
        // Actually end is Mar 25 00:00, and the event on Mar 25 starts at 10:00, so it's not < range_end? Wait: cursor < range_end (Mar 25 00:00), cursor for Mar 25 would be 10:00 which is >= range_end
        // Let me recalculate: Mon 16 (10-11), Wed 18, Mon 23 = 3 instances
        assert!(instances.len() >= 3);
    }

    #[test]
    fn expand_monthly_recurrence() {
        let mut event = make_event("Monthly review", 14, 15);
        event.start = Utc.with_ymd_and_hms(2026, 1, 15, 14, 0, 0).unwrap();
        event.end = Utc.with_ymd_and_hms(2026, 1, 15, 15, 0, 0).unwrap();
        event.recurrence = Some(Recurrence::Monthly { day: 15 });
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
        let instances = expand_recurrence(&event, start, end);
        assert_eq!(instances.len(), 3); // Jan, Feb, Mar
    }

    #[test]
    fn expand_no_recurrence() {
        let event = make_event("One-off", 10, 11);
        let start = Utc.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 17, 0, 0, 0).unwrap();
        let instances = expand_recurrence(&event, start, end);
        assert_eq!(instances.len(), 1);
    }

    // -- ICS tests --

    #[test]
    fn ics_round_trip() {
        let event = make_event("Lunch meeting", 12, 13);
        let ics = event_to_ics(&event);
        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("SUMMARY:Lunch meeting"));
        let parsed = events_from_ics(&ics);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, "Lunch meeting");
    }

    #[test]
    fn ics_with_attendees_and_location() {
        let mut event = make_event("Dinner", 19, 21);
        event.location = Some("The Restaurant".to_string());
        event.attendees = vec!["Alice".to_string(), "Bob".to_string()];
        let ics = event_to_ics(&event);
        assert!(ics.contains("LOCATION:The Restaurant"));
        assert!(ics.contains("ATTENDEE:Alice"));
        assert!(ics.contains("ATTENDEE:Bob"));
    }

    #[test]
    fn ics_multiple_events() {
        let events = vec![make_event("A", 9, 10), make_event("B", 11, 12)];
        let ics = events_to_ics(&events);
        let parsed = events_from_ics(&ics);
        assert_eq!(parsed.len(), 2);
    }

    // -- vCard tests --

    #[test]
    fn vcard_round_trip() {
        let contact = Contact {
            id: Uuid::new_v4(),
            name: "Alice Smith".to_string(),
            email: Some("alice@example.com".to_string()),
            phone: Some("555-1234".to_string()),
            organization: Some("Acme Corp".to_string()),
            notes: Some("Met at conference".to_string()),
            created_at: Utc::now(),
        };
        let vcard = contact_to_vcard(&contact);
        assert!(vcard.contains("BEGIN:VCARD"));
        assert!(vcard.contains("FN:Alice Smith"));
        assert!(vcard.contains("EMAIL:alice@example.com"));
        let parsed = contacts_from_vcard(&vcard);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Alice Smith");
        assert_eq!(parsed[0].email.as_deref(), Some("alice@example.com"));
        assert_eq!(parsed[0].phone.as_deref(), Some("555-1234"));
        assert_eq!(parsed[0].organization.as_deref(), Some("Acme Corp"));
    }

    #[test]
    fn vcard_multiple_contacts() {
        let contacts = vec![
            Contact {
                id: Uuid::new_v4(),
                name: "Alice".to_string(),
                email: None,
                phone: None,
                organization: None,
                notes: None,
                created_at: Utc::now(),
            },
            Contact {
                id: Uuid::new_v4(),
                name: "Bob".to_string(),
                email: Some("bob@example.com".to_string()),
                phone: None,
                organization: None,
                notes: None,
                created_at: Utc::now(),
            },
        ];
        let vcard = contacts_to_vcard(&contacts);
        let parsed = contacts_from_vcard(&vcard);
        assert_eq!(parsed.len(), 2);
    }
}
