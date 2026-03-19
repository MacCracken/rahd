//! Rahd MCP — tool definitions and execution for AGNOS agent integration
//!
//! Provides 5 MCP tools that can be registered with daimon's tool registry,
//! plus an [`execute_tool`] function that runs them against a live store.

use anyhow::Result;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

use rahd_ai::NlEventParser;
use rahd_core::{Event, EventFilter};
use rahd_schedule::Scheduler;
use rahd_store::EventStore;

/// MCP tool description (matches daimon's schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Get the 5 Rahd MCP tool definitions.
pub fn tool_definitions() -> Vec<ToolDescription> {
    vec![
        ToolDescription {
            name: "rahd_events".to_string(),
            description: "List or search calendar events with optional date range and text filter"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "Start date (YYYY-MM-DD)"},
                    "to": {"type": "string", "description": "End date (YYYY-MM-DD)"},
                    "search": {"type": "string", "description": "Text search in event titles"}
                },
                "required": []
            }),
        },
        ToolDescription {
            name: "rahd_add".to_string(),
            description: "Add a new calendar event using natural language or structured parameters"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "description": {"type": "string", "description": "Natural language event description (e.g. 'lunch with Sam tomorrow at noon')"},
                    "title": {"type": "string", "description": "Event title (structured mode)"},
                    "start": {"type": "string", "description": "Start datetime ISO 8601 (structured mode)"},
                    "end": {"type": "string", "description": "End datetime ISO 8601 (structured mode)"}
                },
                "required": []
            }),
        },
        ToolDescription {
            name: "rahd_free".to_string(),
            description: "Find free time slots on a given date within working hours".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "date": {"type": "string", "description": "Date to check (YYYY-MM-DD, default: today)"},
                    "work_start": {"type": "integer", "description": "Work day start hour (default: 9)"},
                    "work_end": {"type": "integer", "description": "Work day end hour (default: 17)"}
                },
                "required": []
            }),
        },
        ToolDescription {
            name: "rahd_conflicts".to_string(),
            description: "Detect scheduling conflicts among upcoming calendar events".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "Start date range (YYYY-MM-DD)"},
                    "to": {"type": "string", "description": "End date range (YYYY-MM-DD)"}
                },
                "required": []
            }),
        },
        ToolDescription {
            name: "rahd_contacts".to_string(),
            description: "List or search contacts by name, email, or organization".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "search": {"type": "string", "description": "Text search across contact fields"},
                    "limit": {"type": "integer", "description": "Maximum results to return (default: 50)"}
                },
                "required": []
            }),
        },
    ]
}

/// Execute an MCP tool by name with the given JSON parameters.
///
/// Returns a JSON response suitable for returning to the agent.
pub fn execute_tool(
    store: &EventStore,
    tool_name: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    match tool_name {
        "rahd_events" => execute_events(store, params),
        "rahd_add" => execute_add(store, params),
        "rahd_free" => execute_free(store, params),
        "rahd_conflicts" => execute_conflicts(store, params),
        "rahd_contacts" => execute_contacts(store, params),
        _ => Err(anyhow::anyhow!("unknown tool: {tool_name}")),
    }
}

fn execute_events(store: &EventStore, params: &serde_json::Value) -> Result<serde_json::Value> {
    let from = params
        .get("from")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<NaiveDate>().ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());
    let to = params
        .get("to")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<NaiveDate>().ok())
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc());
    let search = params
        .get("search")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let filter = EventFilter {
        from,
        to,
        search,
        ..Default::default()
    };
    let events = store.list_events(&filter)?;
    Ok(serde_json::to_value(&events)?)
}

fn execute_add(store: &EventStore, params: &serde_json::Value) -> Result<serde_json::Value> {
    let now = chrono::Utc::now();

    let event = if let Some(desc) = params.get("description").and_then(|v| v.as_str()) {
        // Natural language mode
        let parser = NlEventParser::new();
        let parsed = parser.parse_event(desc)?;
        let (start, end) = parsed.to_datetimes(now);
        Event {
            id: uuid::Uuid::new_v4(),
            title: parsed.title,
            description: Some(desc.to_string()),
            start,
            end,
            location: parsed.location,
            attendees: parsed.attendees,
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: now,
            updated_at: now,
        }
    } else {
        // Structured mode
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let start = params
            .get("start")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or(now);
        let end = params
            .get("end")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or(start + chrono::Duration::hours(1));
        Event {
            id: uuid::Uuid::new_v4(),
            title,
            description: None,
            start,
            end,
            location: None,
            attendees: vec![],
            recurrence: None,
            reminders: vec![],
            calendar_id: "default".to_string(),
            created_at: now,
            updated_at: now,
        }
    };

    store.add_event(&event)?;
    Ok(serde_json::json!({
        "id": event.id.to_string(),
        "title": event.title,
        "start": event.start.to_rfc3339(),
        "end": event.end.to_rfc3339(),
    }))
}

fn execute_free(store: &EventStore, params: &serde_json::Value) -> Result<serde_json::Value> {
    let date = params
        .get("date")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<NaiveDate>().ok())
        .unwrap_or_else(|| Local::now().date_naive());
    let work_start = params
        .get("work_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(9) as u32;
    let work_end = params
        .get("work_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(17) as u32;

    let day_filter = EventFilter {
        from: Some(date.and_hms_opt(0, 0, 0).unwrap().and_utc()),
        to: Some(date.and_hms_opt(23, 59, 59).unwrap().and_utc()),
        ..Default::default()
    };
    let events = store.list_events(&day_filter)?;
    let scheduler = Scheduler::new();
    let slots = scheduler.find_free_slots(&events, date, work_start, work_end);

    let slot_json: Vec<serde_json::Value> = slots
        .iter()
        .map(|s| {
            serde_json::json!({
                "start": s.start.format("%H:%M").to_string(),
                "end": s.end.format("%H:%M").to_string(),
                "duration_mins": s.duration_mins(),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "date": date.to_string(),
        "free_slots": slot_json,
    }))
}

fn execute_conflicts(store: &EventStore, params: &serde_json::Value) -> Result<serde_json::Value> {
    let from = params
        .get("from")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<NaiveDate>().ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());
    let to = params
        .get("to")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<NaiveDate>().ok())
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc());

    let filter = EventFilter {
        from,
        to,
        ..Default::default()
    };
    let events = store.list_events(&filter)?;
    let scheduler = Scheduler::new();
    let conflicts = scheduler.find_conflicts(&events);

    Ok(serde_json::to_value(&conflicts)?)
}

fn execute_contacts(store: &EventStore, params: &serde_json::Value) -> Result<serde_json::Value> {
    let search = params
        .get("search")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let mut contacts = store.list_contacts()?;
    if let Some(ref q) = search {
        contacts.retain(|c| {
            c.name.to_lowercase().contains(q)
                || c.email
                    .as_ref()
                    .is_some_and(|e| e.to_lowercase().contains(q))
                || c.organization
                    .as_ref()
                    .is_some_and(|o| o.to_lowercase().contains(q))
        });
    }
    contacts.truncate(limit);
    Ok(serde_json::to_value(&contacts)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definitions_count() {
        assert_eq!(tool_definitions().len(), 5);
    }

    #[test]
    fn tool_names() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"rahd_events"));
        assert!(names.contains(&"rahd_add"));
        assert!(names.contains(&"rahd_free"));
        assert!(names.contains(&"rahd_conflicts"));
        assert!(names.contains(&"rahd_contacts"));
    }

    #[test]
    fn tool_schemas_valid_json() {
        for tool in tool_definitions() {
            assert!(tool.input_schema.is_object());
        }
    }

    #[test]
    fn execute_events_empty_store() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(&store, "rahd_events", &serde_json::json!({})).unwrap();
        assert!(result.as_array().unwrap().is_empty());
    }

    #[test]
    fn execute_add_natural_language() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(
            &store,
            "rahd_add",
            &serde_json::json!({"description": "lunch tomorrow at noon"}),
        )
        .unwrap();
        assert!(result.get("id").is_some());
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "lunch");
    }

    #[test]
    fn execute_add_structured() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(
            &store,
            "rahd_add",
            &serde_json::json!({
                "title": "Team sync",
                "start": "2026-03-20T10:00:00Z",
                "end": "2026-03-20T11:00:00Z"
            }),
        )
        .unwrap();
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "Team sync");
    }

    #[test]
    fn execute_free_empty() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(
            &store,
            "rahd_free",
            &serde_json::json!({"date": "2026-03-20"}),
        )
        .unwrap();
        let slots = result.get("free_slots").unwrap().as_array().unwrap();
        assert_eq!(slots.len(), 1); // One big free slot (9-17)
    }

    #[test]
    fn execute_conflicts_empty() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(&store, "rahd_conflicts", &serde_json::json!({})).unwrap();
        assert!(result.as_array().unwrap().is_empty());
    }

    #[test]
    fn execute_contacts_empty() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(&store, "rahd_contacts", &serde_json::json!({})).unwrap();
        assert!(result.as_array().unwrap().is_empty());
    }

    #[test]
    fn execute_unknown_tool() {
        let store = EventStore::new_in_memory().unwrap();
        let result = execute_tool(&store, "unknown", &serde_json::json!({}));
        assert!(result.is_err());
    }
}
