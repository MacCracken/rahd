//! Agnoshi intent resolver — deterministic routing for common scheduling queries.
//!
//! Maps natural language patterns to MCP tool calls without requiring hoosh (LLM).
//! Falls back to hoosh when the intent is ambiguous or unrecognized.

use chrono::{Datelike, Duration, Local, NaiveDate};
use rahd_core::EventFilter;
use rahd_mcp::execute_tool;
use rahd_store::EventStore;

use crate::hoosh::IntentAction;

/// Result of resolving an intent locally.
pub struct ResolvedIntent {
    pub action: IntentAction,
    pub explanation: String,
    pub result: serde_json::Value,
}

/// Try to resolve a scheduling intent locally without LLM assistance.
/// Returns `None` if the query is too ambiguous for deterministic resolution.
pub fn resolve_intent(store: &EventStore, query: &str) -> Option<ResolvedIntent> {
    let lower = query.to_lowercase();

    if is_show_schedule(&lower) {
        return Some(handle_show_schedule(store, &lower));
    }

    if is_find_free_time(&lower) {
        return Some(handle_find_free_time(store, &lower));
    }

    if is_schedule_meeting(&lower) {
        return handle_schedule_meeting(store, query);
    }

    None
}

fn is_show_schedule(query: &str) -> bool {
    let patterns = [
        "show my week",
        "show my schedule",
        "show my calendar",
        "what's on my calendar",
        "what do i have",
        "what's my schedule",
        "what are my events",
        "show my day",
        "show today",
        "show this week",
        "my schedule",
        "my events",
        "my calendar",
        "what's on today",
        "what's happening",
    ];
    patterns.iter().any(|p| query.contains(p))
}

fn is_find_free_time(query: &str) -> bool {
    let patterns = [
        "when am i free",
        "when are you free",
        "free time",
        "free slots",
        "available time",
        "availability",
        "open slots",
        "when can i",
    ];
    patterns.iter().any(|p| query.contains(p))
}

fn is_schedule_meeting(query: &str) -> bool {
    let patterns = [
        "schedule a meeting",
        "schedule meeting",
        "set up a meeting",
        "book a meeting",
        "create a meeting",
        "add a meeting",
        "schedule an event",
        "add an event",
        "create an event",
        "book an event",
    ];
    patterns.iter().any(|p| query.contains(p))
}

fn handle_show_schedule(store: &EventStore, query: &str) -> ResolvedIntent {
    let today = Local::now().date_naive();

    let (from, to, period_label) = if query.contains("week") || query.contains("this week") {
        let weekday = today.weekday().num_days_from_monday();
        let monday = today - Duration::days(weekday as i64);
        let sunday = monday + Duration::days(6);
        (monday, sunday, "this week")
    } else if query.contains("tomorrow") {
        let tomorrow = today + Duration::days(1);
        (tomorrow, tomorrow, "tomorrow")
    } else {
        // Default to today
        (today, today, "today")
    };

    let filter = EventFilter {
        from: Some(from.and_hms_opt(0, 0, 0).unwrap().and_utc()),
        to: Some(to.and_hms_opt(23, 59, 59).unwrap().and_utc()),
        ..Default::default()
    };

    let events = store.list_events(&filter).unwrap_or_default();
    let events_json: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "title": e.title,
                "start": e.start.to_rfc3339(),
                "end": e.end.to_rfc3339(),
                "location": e.location,
            })
        })
        .collect();

    let explanation = if events.is_empty() {
        format!("You have no events {period_label}.")
    } else {
        format!(
            "You have {} event{} {}.",
            events.len(),
            if events.len() == 1 { "" } else { "s" },
            period_label
        )
    };

    ResolvedIntent {
        action: IntentAction::ShowSchedule,
        explanation,
        result: serde_json::json!({
            "period": period_label,
            "from": from.to_string(),
            "to": to.to_string(),
            "events": events_json,
        }),
    }
}

fn handle_find_free_time(store: &EventStore, query: &str) -> ResolvedIntent {
    let today = Local::now().date_naive();

    let date = if query.contains("tomorrow") {
        today + Duration::days(1)
    } else {
        // Try to find a date in the query, default to today
        extract_date(query).unwrap_or(today)
    };

    let params = serde_json::json!({"date": date.to_string()});
    let result = execute_tool(store, "rahd_free", &params)
        .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}));

    let slots = result
        .get("free_slots")
        .and_then(|s| s.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let explanation = if slots == 0 {
        format!("No free slots on {date}.")
    } else {
        format!(
            "You have {slots} free slot{} on {date}.",
            if slots == 1 { "" } else { "s" }
        )
    };

    ResolvedIntent {
        action: IntentAction::FindFreeTime,
        explanation,
        result,
    }
}

fn handle_schedule_meeting(store: &EventStore, query: &str) -> Option<ResolvedIntent> {
    // Use the NL parser to extract event details
    let parser = rahd_ai::NlEventParser::new();
    let parsed = parser.parse_event(query).ok()?;

    let now = chrono::Utc::now();
    let (start, end) = parsed.to_datetimes(now);

    let params = serde_json::json!({
        "title": parsed.title,
        "start": start.to_rfc3339(),
        "end": end.to_rfc3339(),
    });

    let result = execute_tool(store, "rahd_add", &params).ok()?;

    let explanation = format!(
        "Scheduled \"{}\" from {} to {}.",
        parsed.title,
        start.with_timezone(&Local).format("%a %b %-d at %-I:%M %p"),
        end.with_timezone(&Local).format("%-I:%M %p"),
    );

    Some(ResolvedIntent {
        action: IntentAction::CreateEvent,
        explanation,
        result,
    })
}

/// Try to extract a date from a query string.
fn extract_date(query: &str) -> Option<NaiveDate> {
    let today = Local::now().date_naive();
    if query.contains("tomorrow") {
        return Some(today + Duration::days(1));
    }
    if query.contains("today") {
        return Some(today);
    }
    // Try to find a YYYY-MM-DD pattern
    for word in query.split_whitespace() {
        if let Ok(d) = word.parse::<NaiveDate>() {
            return Some(d);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> EventStore {
        EventStore::new_in_memory().unwrap()
    }

    #[test]
    fn resolve_show_schedule_today() {
        let store = test_store();
        let result = resolve_intent(&store, "show my schedule").unwrap();
        assert_eq!(result.action, IntentAction::ShowSchedule);
        assert!(result.result.get("events").is_some());
        assert!(result.result["period"] == "today");
    }

    #[test]
    fn resolve_show_schedule_week() {
        let store = test_store();
        let result = resolve_intent(&store, "show my week").unwrap();
        assert_eq!(result.action, IntentAction::ShowSchedule);
        assert!(result.result["period"] == "this week");
    }

    #[test]
    fn resolve_find_free_time() {
        let store = test_store();
        let result = resolve_intent(&store, "when am I free tomorrow").unwrap();
        assert_eq!(result.action, IntentAction::FindFreeTime);
        assert!(result.result.get("free_slots").is_some());
    }

    #[test]
    fn resolve_schedule_meeting() {
        let store = test_store();
        let result =
            resolve_intent(&store, "schedule a meeting with Alice tomorrow at 3pm").unwrap();
        assert_eq!(result.action, IntentAction::CreateEvent);
        assert!(result.result.get("id").is_some());
    }

    #[test]
    fn resolve_unknown_returns_none() {
        let store = test_store();
        assert!(resolve_intent(&store, "what is the meaning of life").is_none());
    }

    #[test]
    fn resolve_whats_on_my_calendar() {
        let store = test_store();
        let result = resolve_intent(&store, "what's on my calendar").unwrap();
        assert_eq!(result.action, IntentAction::ShowSchedule);
    }

    #[test]
    fn resolve_when_can_i() {
        let store = test_store();
        let result = resolve_intent(&store, "when can i meet someone").unwrap();
        assert_eq!(result.action, IntentAction::FindFreeTime);
    }
}
