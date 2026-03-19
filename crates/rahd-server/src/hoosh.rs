//! hoosh API client — LLM-powered scheduling via the hoosh service (port 8088).
//!
//! Provides a client that forwards scheduling intents to hoosh and parses
//! the LLM responses back into structured calendar operations.

use serde::{Deserialize, Serialize};

/// hoosh API client.
#[derive(Clone)]
pub struct HooshClient {
    base_url: String,
    client: reqwest::Client,
}

/// A scheduling intent sent to hoosh for LLM processing.
#[derive(Debug, Serialize, Deserialize)]
pub struct SchedulingIntent {
    /// The natural language request (e.g. "schedule a meeting with Alice next week").
    pub query: String,
    /// Current events context for the LLM to reason about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<IntentContext>,
}

/// Context provided alongside an intent for better LLM reasoning.
#[derive(Debug, Serialize, Deserialize)]
pub struct IntentContext {
    /// Existing events in the relevant time range.
    pub events: Vec<serde_json::Value>,
    /// Free time slots available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_slots: Option<Vec<serde_json::Value>>,
    /// Current date/time for relative references.
    pub now: String,
}

/// Response from hoosh after processing a scheduling intent.
#[derive(Debug, Serialize, Deserialize)]
pub struct HooshResponse {
    /// The suggested action (e.g. "create_event", "reschedule", "show_schedule").
    pub action: String,
    /// Human-readable explanation from the LLM.
    pub explanation: String,
    /// Structured parameters for the action (event details, time ranges, etc.).
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Known agnoshi intent types that rahd handles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    /// Create a new event.
    CreateEvent,
    /// Reschedule an existing event.
    Reschedule,
    /// Show the user's schedule.
    ShowSchedule,
    /// Find free time.
    FindFreeTime,
    /// Cancel/delete an event.
    CancelEvent,
    /// Unknown/unsupported action.
    Unknown,
}

impl From<&str> for IntentAction {
    fn from(s: &str) -> Self {
        match s {
            "create_event" => IntentAction::CreateEvent,
            "reschedule" => IntentAction::Reschedule,
            "show_schedule" => IntentAction::ShowSchedule,
            "find_free_time" => IntentAction::FindFreeTime,
            "cancel_event" => IntentAction::CancelEvent,
            _ => IntentAction::Unknown,
        }
    }
}

impl HooshClient {
    /// Create a new hoosh client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Default client pointing at localhost:8088.
    pub fn default_local() -> Self {
        Self::new("http://127.0.0.1:8088")
    }

    /// Check if hoosh is reachable.
    pub async fn health(&self) -> anyhow::Result<bool> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await;
        Ok(resp.is_ok_and(|r| r.status().is_success()))
    }

    /// Send a scheduling intent to hoosh for LLM processing.
    pub async fn schedule(&self, intent: &SchedulingIntent) -> anyhow::Result<HooshResponse> {
        let resp = self
            .client
            .post(format!("{}/v1/intent", self.base_url))
            .json(intent)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("hoosh returned {status}: {body}");
        }

        let result = resp.json::<HooshResponse>().await?;
        Ok(result)
    }

    /// Convenience: send a plain text query with auto-populated context.
    pub async fn query(
        &self,
        query: &str,
        context: Option<IntentContext>,
    ) -> anyhow::Result<HooshResponse> {
        let intent = SchedulingIntent {
            query: query.to_string(),
            context,
        };
        self.schedule(&intent).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_action_from_str() {
        assert_eq!(
            IntentAction::from("create_event"),
            IntentAction::CreateEvent
        );
        assert_eq!(IntentAction::from("reschedule"), IntentAction::Reschedule);
        assert_eq!(
            IntentAction::from("show_schedule"),
            IntentAction::ShowSchedule
        );
        assert_eq!(
            IntentAction::from("find_free_time"),
            IntentAction::FindFreeTime
        );
        assert_eq!(
            IntentAction::from("cancel_event"),
            IntentAction::CancelEvent
        );
        assert_eq!(IntentAction::from("gibberish"), IntentAction::Unknown);
    }

    #[test]
    fn scheduling_intent_serialization() {
        let intent = SchedulingIntent {
            query: "schedule a meeting with Alice tomorrow at 2pm".to_string(),
            context: None,
        };
        let json = serde_json::to_value(&intent).unwrap();
        assert_eq!(
            json["query"],
            "schedule a meeting with Alice tomorrow at 2pm"
        );
        assert!(json.get("context").is_none());
    }

    #[test]
    fn hoosh_response_deserialization() {
        let json = serde_json::json!({
            "action": "create_event",
            "explanation": "I'll schedule a meeting with Alice tomorrow at 2:00 PM.",
            "params": {
                "title": "Meeting with Alice",
                "start": "2026-03-19T14:00:00Z",
                "end": "2026-03-19T15:00:00Z"
            }
        });
        let resp: HooshResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.action, "create_event");
        assert_eq!(
            IntentAction::from(resp.action.as_str()),
            IntentAction::CreateEvent
        );
        assert_eq!(resp.params["title"], "Meeting with Alice");
    }

    #[test]
    fn hoosh_client_default_url() {
        let client = HooshClient::default_local();
        assert_eq!(client.base_url, "http://127.0.0.1:8088");
    }
}
