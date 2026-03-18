//! Rahd Server — daimon API server exposing MCP tools over HTTP
//!
//! Runs on port 8090 and provides:
//! - `GET /health` — health check
//! - `GET /tools` — list MCP tool definitions
//! - `POST /tools/{tool_name}` — execute an MCP tool

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use rahd_mcp::{execute_tool, tool_definitions};
use rahd_store::EventStore;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    store: Arc<Mutex<EventStore>>,
}

impl AppState {
    pub fn new(store: EventStore) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
        }
    }
}

/// Build the axum router with all routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tools", get(list_tools))
        .route("/tools/{tool_name}", post(call_tool))
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

async fn list_tools() -> Json<serde_json::Value> {
    let tools = tool_definitions();
    Json(serde_json::to_value(&tools).unwrap())
}

async fn call_tool(
    State(state): State<AppState>,
    Path(tool_name): Path<String>,
    Json(params): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let store = state.store.lock().unwrap();
    match execute_tool(&store, &tool_name, &params) {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
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
        assert_eq!(resp.status(), StatusCode::OK);
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
