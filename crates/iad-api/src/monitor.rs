//! Real-time monitoring endpoints — REST + SSE (Server-Sent Events).
//!
//! Endpoints:
//!   GET /api/v1/monitor/dashboard        — JSON dashboard snapshot
//!   GET /api/v1/monitor/agents           — all agent states
//!   GET /api/v1/monitor/agents/:name     — single agent state
//!   GET /api/v1/monitor/agents/:name/stream — SSE stream for one agent
//!   GET /api/v1/monitor/stream           — SSE stream for ALL agents (live events)

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{sse, IntoResponse, Json, Sse},
    routing::get,
    Router,
};
use futures::stream::{self, Stream};
use iad_core::monitor::Monitor;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use crate::{AppState, ErrorResp};

/// Build the /api/v1/monitor sub-router.
pub fn monitor_router() -> Router<AppState> {
    Router::new()
        .route("/dashboard",          get(dashboard))
        .route("/agents",             get(all_agents))
        .route("/agents/:name",       get(agent_state))
        .route("/agents/:name/stream", get(agent_sse))
        .route("/stream",             get(all_agents_sse))
}

#[derive(Deserialize)]
pub struct SseQuery {
    /// Polling interval in ms for SSE snapshots (default 500).
    pub interval_ms: Option<u64>,
    pub token:       Option<String>,
}

fn check_auth_monitor(headers: &HeaderMap, query_token: &Option<String>, api_token: &Option<String>) -> bool {
    match api_token {
        None => true,
        Some(expected) => {
            // Check header first
            let from_header = headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|t| t == expected)
                .unwrap_or(false);
            // Fall back to query param
            let from_query = query_token.as_deref().map(|t| t == expected).unwrap_or(false);
            from_header || from_query
        }
    }
}

// ─── Dashboard ───────────────────────────────────────────────────────────────

async fn dashboard(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> axum::response::Response {
    if !check_auth_monitor(&headers, &None, &state.api_token) {
        return (StatusCode::UNAUTHORIZED, Json(ErrorResp::new("unauthorized"))).into_response();
    }
    state.metrics.inc_api_requests();
    match &state.monitor {
        Some(mon) => Json(mon.dashboard().await).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResp::new("monitor not enabled"))).into_response(),
    }
}

// ─── All Agents State ────────────────────────────────────────────────────────

async fn all_agents(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> axum::response::Response {
    if !check_auth_monitor(&headers, &None, &state.api_token) {
        return (StatusCode::UNAUTHORIZED, Json(ErrorResp::new("unauthorized"))).into_response();
    }
    state.metrics.inc_api_requests();
    match &state.monitor {
        Some(mon) => {
            let snap = mon.dashboard().await;
            Json(snap.agents).into_response()
        }
        None => (StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResp::new("monitor not enabled"))).into_response(),
    }
}

// ─── Single Agent State ──────────────────────────────────────────────────────

async fn agent_state(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> axum::response::Response {
    if !check_auth_monitor(&headers, &None, &state.api_token) {
        return (StatusCode::UNAUTHORIZED, Json(ErrorResp::new("unauthorized"))).into_response();
    }
    state.metrics.inc_api_requests();
    match &state.monitor {
        Some(mon) => match mon.get_agent(&name).await {
            Some(a) => Json(a).into_response(),
            None => (StatusCode::NOT_FOUND,
                Json(ErrorResp::new(&format!("agent '{name}' not monitored")))).into_response(),
        },
        None => (StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResp::new("monitor not enabled"))).into_response(),
    }
}

// ─── SSE: Single Agent Stream ────────────────────────────────────────────────

async fn agent_sse(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<SseQuery>,
) -> axum::response::Response {
    if !check_auth_monitor(&headers, &query.token, &state.api_token) {
        return (StatusCode::UNAUTHORIZED, Json(ErrorResp::new("unauthorized"))).into_response();
    }

    let Some(mon) = state.monitor.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResp::new("monitor not enabled"))).into_response();
    };

    let interval = Duration::from_millis(query.interval_ms.unwrap_or(500).max(100).min(5000));
    let agent_name = name.clone();

    let stream = async_stream::stream! {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            match mon.get_agent(&agent_name).await {
                Some(state) => {
                    let json = serde_json::to_string(&state).unwrap_or_default();
                    yield Ok::<_, Infallible>(sse::Event::default()
                        .event("agent_state")
                        .data(json));
                }
                None => {
                    yield Ok(sse::Event::default()
                        .event("agent_not_found")
                        .data(format!("{{\"agent\":\"{agent_name}\",\"error\":\"not found\"}}")));
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(sse::KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

// ─── SSE: All Agents Dashboard Stream ────────────────────────────────────────

async fn all_agents_sse(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SseQuery>,
) -> axum::response::Response {
    if !check_auth_monitor(&headers, &query.token, &state.api_token) {
        return (StatusCode::UNAUTHORIZED, Json(ErrorResp::new("unauthorized"))).into_response();
    }

    let Some(mon) = state.monitor.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResp::new("monitor not enabled"))).into_response();
    };

    let interval = Duration::from_millis(query.interval_ms.unwrap_or(500).max(100).min(5000));

    let stream = async_stream::stream! {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let snap = mon.dashboard().await;
            let json = serde_json::to_string(&snap).unwrap_or_default();
            yield Ok::<_, Infallible>(sse::Event::default()
                .event("dashboard")
                .data(json));
        }
    };

    Sse::new(stream)
        .keep_alive(sse::KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}
