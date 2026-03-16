//! Rahd Schedule — conflict detection, free/busy analysis, and time slot suggestions

use chrono::NaiveDate;
use rahd_core::{Conflict, Event, TimeSlot};

/// Scheduling engine for calendar events.
pub struct Scheduler;

impl Scheduler {
    pub fn new() -> Self {
        Self
    }

    /// Check if two events overlap.
    pub fn is_overlapping(a: &Event, b: &Event) -> bool {
        a.start < b.end && b.start < a.end
    }

    /// Find all scheduling conflicts among the given events. O(n^2).
    pub fn find_conflicts(&self, events: &[Event]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        for i in 0..events.len() {
            for j in (i + 1)..events.len() {
                if Self::is_overlapping(&events[i], &events[j]) {
                    let overlap_start = events[i].start.max(events[j].start);
                    let overlap_end = events[i].end.min(events[j].end);
                    conflicts.push(Conflict {
                        event_a: events[i].id,
                        event_b: events[j].id,
                        overlap: TimeSlot {
                            start: overlap_start,
                            end: overlap_end,
                        },
                    });
                }
            }
        }
        conflicts
    }

    /// Find free time slots on a given date within working hours.
    ///
    /// `work_start` and `work_end` are hours (e.g. 9 and 17).
    pub fn find_free_slots(
        &self,
        events: &[Event],
        date: NaiveDate,
        work_start: u32,
        work_end: u32,
    ) -> Vec<TimeSlot> {
        let day_start = date.and_hms_opt(work_start, 0, 0).unwrap().and_utc();
        let day_end = date.and_hms_opt(work_end, 0, 0).unwrap().and_utc();

        // Collect events that fall on this day, clipped to work hours.
        let mut busy: Vec<TimeSlot> = events
            .iter()
            .filter(|e| e.start < day_end && e.end > day_start)
            .map(|e| TimeSlot {
                start: e.start.max(day_start),
                end: e.end.min(day_end),
            })
            .collect();

        busy.sort_by_key(|s| s.start);

        let mut free = Vec::new();
        let mut cursor = day_start;

        for slot in &busy {
            if slot.start > cursor {
                free.push(TimeSlot {
                    start: cursor,
                    end: slot.start,
                });
            }
            if slot.end > cursor {
                cursor = slot.end;
            }
        }

        if cursor < day_end {
            free.push(TimeSlot {
                start: cursor,
                end: day_end,
            });
        }

        free
    }

    /// Suggest the first available meeting slot of the given duration on a date.
    pub fn suggest_meeting_time(
        &self,
        events: &[Event],
        duration_mins: u32,
        date: NaiveDate,
    ) -> Option<TimeSlot> {
        let free = self.find_free_slots(events, date, 9, 17);
        let duration = chrono::Duration::minutes(duration_mins as i64);
        for slot in &free {
            if slot.end - slot.start >= duration {
                return Some(TimeSlot {
                    start: slot.start,
                    end: slot.start + duration,
                });
            }
        }
        None
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn make_event(title: &str, start_hour: u32, end_hour: u32) -> Event {
        let now = Utc::now();
        Event {
            id: Uuid::new_v4(),
            title: title.to_string(),
            description: None,
            start: Utc.with_ymd_and_hms(2026, 3, 16, start_hour, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 3, 16, end_hour, 0, 0).unwrap(),
            location: None,
            attendees: vec![],
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn no_conflicts_no_overlap() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("A", 9, 10), make_event("B", 11, 12)];
        assert!(scheduler.find_conflicts(&events).is_empty());
    }

    #[test]
    fn detect_overlap() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("A", 9, 11), make_event("B", 10, 12)];
        let conflicts = scheduler.find_conflicts(&events);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn no_overlap_back_to_back() {
        assert!(!Scheduler::is_overlapping(
            &make_event("A", 9, 10),
            &make_event("B", 10, 11),
        ));
    }

    #[test]
    fn same_start_time_conflicts() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("A", 10, 11), make_event("B", 10, 12)];
        assert_eq!(scheduler.find_conflicts(&events).len(), 1);
    }

    #[test]
    fn free_slots_empty_day() {
        let scheduler = Scheduler::new();
        let date = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        let slots = scheduler.find_free_slots(&[], date, 9, 17);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].duration_mins(), 480); // 8 hours
    }

    #[test]
    fn free_slots_with_events() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("Meeting", 10, 11), make_event("Lunch", 12, 13)];
        let date = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        let slots = scheduler.find_free_slots(&events, date, 9, 17);
        // 9-10, 11-12, 13-17
        assert_eq!(slots.len(), 3);
    }

    #[test]
    fn free_slots_full_day() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("All day", 9, 17)];
        let date = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        let slots = scheduler.find_free_slots(&events, date, 9, 17);
        assert!(slots.is_empty());
    }

    #[test]
    fn suggest_meeting_time_empty_day() {
        let scheduler = Scheduler::new();
        let date = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        let slot = scheduler.suggest_meeting_time(&[], 60, date).unwrap();
        assert_eq!(slot.start.hour(), 9);
        assert_eq!(slot.duration_mins(), 60);
    }

    #[test]
    fn suggest_meeting_time_with_events() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("Standup", 9, 10)];
        let date = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        let slot = scheduler.suggest_meeting_time(&events, 60, date).unwrap();
        assert_eq!(slot.start.hour(), 10);
    }

    #[test]
    fn suggest_meeting_time_no_room() {
        let scheduler = Scheduler::new();
        let events = vec![make_event("All day", 9, 17)];
        let date = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        assert!(scheduler.suggest_meeting_time(&events, 60, date).is_none());
    }

    use chrono::Timelike;
}
