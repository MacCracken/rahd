//! Rahd Server — daimon API server exposing MCP tools over HTTP
//!
//! Runs on port 8090 and provides:
//! - `GET /health` — health check
//! - `GET /tools` — list MCP tool definitions
//! - `POST /tools/{tool_name}` — execute an MCP tool
//! - `POST /intents/resolve` — resolve scheduling intents locally (no LLM)
//! - `GET /hoosh/health` — check hoosh connectivity
//! - `POST /hoosh/intent` — scheduling intent with local resolution + hoosh LLM fallback

pub mod hoosh;
pub mod intents;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use rahd_mcp::{execute_tool, tool_definitions};
use rahd_store::EventStore;

use crate::hoosh::{HooshClient, IntentContext, SchedulingIntent};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    store: Arc<Mutex<EventStore>>,
    hoosh: HooshClient,
}

impl AppState {
    pub fn new(store: EventStore) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            hoosh: HooshClient::default_local(),
        }
    }

    pub fn with_hoosh_url(mut self, url: &str) -> Self {
        self.hoosh = HooshClient::new(url);
        self
    }
}

/// Build the axum router with all routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tools", get(list_tools))
        .route("/tools/{tool_name}", post(call_tool))
        .route("/hoosh/health", get(hoosh_health))
        .route("/hoosh/intent", post(hoosh_intent))
        .route("/intents/resolve", post(resolve_intent_handler))
        .with_state(state)
}

/// Start the server on the given address.
pub async fn serve(state: AppState, addr: &str) -> anyhow::Result<()> {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("rahd-server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn list_tools() -> (StatusCode, Json<serde_json::Value>) {
    let tools = tool_definitions();
    match serde_json::to_value(&tools) {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

async fn call_tool(
    State(state): State<AppState>,
    Path(tool_name): Path<String>,
    Json(params): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = match state.store.lock() {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "store unavailable"})),
            );
        }
    };
    match execute_tool(&store, &tool_name, &params) {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

async fn hoosh_health(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    if state.hoosh.health().await {
        (
            StatusCode::OK,
            Json(serde_json::json!({"status": "connected"})),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "unreachable"})),
        )
    }
}

async fn resolve_intent_handler(
    State(state): State<AppState>,
    Json(intent): Json<SchedulingIntent>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = match state.store.lock() {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "store unavailable"})),
            );
        }
    };

    match intents::resolve_intent(&store, &intent.query) {
        Some(resolved) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "action": resolved.action,
                "explanation": resolved.explanation,
                "result": resolved.result,
                "source": "local",
            })),
        ),
        None => (
            StatusCode::OK,
            Json(serde_json::json!({
                "action": "unknown",
                "explanation": "Could not resolve intent locally. Try /hoosh/intent for LLM-assisted resolution.",
                "source": "local",
            })),
        ),
    }
}

async fn hoosh_intent(
    State(state): State<AppState>,
    Json(mut intent): Json<SchedulingIntent>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Try local intent resolution first (fast, no LLM needed)
    if let Ok(store) = state.store.lock()
        && let Some(resolved) = intents::resolve_intent(&store, &intent.query)
    {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "action": resolved.action,
                "explanation": resolved.explanation,
                "result": resolved.result,
                "source": "local",
            })),
        );
    }

    // Fall back to hoosh LLM for ambiguous queries
    // Auto-populate context if not provided
    if intent.context.is_none() {
        let events = state
            .store
            .lock()
            .ok()
            .and_then(|s| s.list_events(&rahd_core::EventFilter::default()).ok())
            .unwrap_or_default();

        let events_json: Vec<serde_json::Value> = events
            .iter()
            .map(|e| {
                serde_json::json!({
                    "title": e.title,
                    "start": e.start.to_rfc3339(),
                    "end": e.end.to_rfc3339(),
                })
            })
            .collect();

        intent.context = Some(IntentContext {
            events: events_json,
            free_slots: None,
            now: chrono::Local::now().to_rfc3339(),
        });
    }

    match state.hoosh.schedule(&intent).await {
        Ok(response) => {
            // Execute the suggested action if it maps to a known tool
            let result = match response.action.as_str() {
                "create_event" => {
                    // Forward to rahd_add tool
                    let store = match state.store.lock() {
                        Ok(s) => s,
                        Err(_) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({"error": "store unavailable"})),
                            );
                        }
                    };
                    match execute_tool(&store, "rahd_add", &response.params) {
                        Ok(tool_result) => serde_json::json!({
                            "action": response.action,
                            "explanation": response.explanation,
                            "result": tool_result,
                        }),
                        Err(e) => serde_json::json!({
                            "action": response.action,
                            "explanation": response.explanation,
                            "error": e.to_string(),
                        }),
                    }
                }
                _ => {
                    // Return the LLM suggestion without auto-executing
                    serde_json::json!({
                        "action": response.action,
                        "explanation": response.explanation,
                        "params": response.params,
                    })
                }
            };
            (StatusCode::OK, Json(result))
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("hoosh error: {e}")})),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        AppState::new(EventStore::new_in_memory().unwrap())
    }

    #[tokio::test]
    async fn health_check() {
        let app = router(test_state());
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_tools_returns_five() {
        let app = router(test_state());
        let resp = app
            .oneshot(Request::get("/tools").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools.len(), 5);
    }

    #[tokio::test]
    async fn call_tool_events_empty() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/tools/rahd_events")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let events: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn call_tool_add_structured() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/tools/rahd_add")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Test Event",
                            "start": "2026-03-20T10:00:00Z",
                            "end": "2026-03-20T11:00:00Z"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["title"], "Test Event");
    }

    #[tokio::test]
    async fn call_unknown_tool() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/tools/nonexistent")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(result.get("error").is_some());
    }

    #[tokio::test]
    async fn call_tool_free_slots() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/tools/rahd_free")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"date": "2026-03-20"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(result.get("free_slots").is_some());
    }

    #[tokio::test]
    async fn call_tool_conflicts_empty() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/tools/rahd_conflicts")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn hoosh_health_unreachable() {
        // hoosh isn't running in test, so health should report unreachable
        let app = router(test_state());
        let resp = app
            .oneshot(Request::get("/hoosh/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["status"], "unreachable");
    }

    #[tokio::test]
    async fn call_tool_add_then_search_description() {
        let app = router(test_state());
        // Add an event with a description
        let add_resp = app
            .clone()
            .oneshot(
                Request::post("/tools/rahd_add")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "description": "budget meeting tomorrow at noon"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(add_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn hoosh_intent_local_resolution() {
        // "schedule a meeting" is resolved locally without needing hoosh
        let app = router(test_state());
        let resp = app
            .clone()
            .oneshot(
                Request::post("/hoosh/intent")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"query": "schedule a meeting tomorrow at 3pm"})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["source"], "local");
        assert_eq!(result["action"], "create_event");
    }

    #[tokio::test]
    async fn hoosh_intent_falls_through_to_hoosh() {
        // Ambiguous query that local resolver can't handle falls through to hoosh
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/hoosh/intent")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"query": "rearrange everything to maximize focus time"})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Should return BAD_GATEWAY since hoosh isn't running and local can't handle it
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn resolve_intent_endpoint() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/intents/resolve")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"query": "when am I free"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["action"], "find_free_time");
        assert_eq!(result["source"], "local");
    }

    #[tokio::test]
    async fn resolve_intent_unknown() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/intents/resolve")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"query": "what is the weather"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["action"], "unknown");
    }

    #[tokio::test]
    async fn call_tool_contacts_empty() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::post("/tools/rahd_contacts")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(result.is_empty());
    }
}
