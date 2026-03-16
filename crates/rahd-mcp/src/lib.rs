//! Rahd MCP — tool definitions for AGNOS agent integration
//!
//! 5 native tools that can be registered with daimon's MCP tool registry.

use serde::{Deserialize, Serialize};

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
}
