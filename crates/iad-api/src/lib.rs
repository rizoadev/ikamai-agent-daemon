//! IKAMAI Agent Daemon — REST API server (Phase 3.1-3.11).

pub mod alerts;
pub mod metrics;
pub mod ws;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use iad_core::{AgentRegistry, AgentStatus, DaemonConfig, event_bus::EventBus};
use iad_storage::Storage;
use metrics::Metrics;
use alerts::AlertManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ─── App State ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub storage:   Storage,
    pub registry:  Arc<AgentRegistry>,
    pub cfg:       DaemonConfig,
    pub api_token: Option<String>,
    pub metrics:   Arc<Metrics>,
    pub event_bus: Arc<EventBus>,
    pub alert_mgr: Option<Arc<AlertManager>>,
}

// ─── Router ──────────────────────────────────────────────────────────────────

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health & system
        .route("/healthz",                              get(healthz))
        .route("/metrics",                              get(metrics::metrics_handler))
        .route("/api/v1/status",                        get(system_status))
        .route("/api/v1/reload",                        post(system_reload))
        // WebSocket
        .route("/api/v1/ws",                            get(ws::ws_handler))
        // Agents
        .route("/api/v1/agents",                        get(list_agents).post(register_agent))
        .route("/api/v1/agents/:name",                  get(get_agent).delete(delete_agent))
        .route("/api/v1/agents/:name/start",            post(start_agent))
        .route("/api/v1/agents/:name/stop",             post(stop_agent))
        .route("/api/v1/agents/:name/logs",             get(agent_logs))
        // Tasks
        .route("/api/v1/tasks",                         get(list_tasks).post(submit_task))
        .route("/api/v1/tasks/dead-letter",             get(list_dead_letter))
        .route("/api/v1/tasks/dead-letter/:id/retry",   post(retry_dead_letter))
        .route("/api/v1/tasks/:id",                     get(get_task).delete(cancel_task))
        .with_state(state)
}

// ─── Auth ────────────────────────────────────────────────────────────────────

fn check_auth(headers: &HeaderMap, token: &Option<String>) -> bool {
    match token {
        None => true,
        Some(expected) => headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|t| t == expected)
            .unwrap_or(false),
    }
}

// ─── Response types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ErrorResp { pub error: String }
impl ErrorResp { pub fn new(m: &str) -> Self { Self { error: m.into() } } }

#[derive(Serialize)]
pub struct OkResp { pub ok: bool, #[serde(skip_serializing_if = "Option::is_none")] pub message: Option<String> }
impl OkResp {
    pub fn ok()      -> Self { Self { ok: true, message: None } }
    pub fn msg(m: &str) -> Self { Self { ok: true, message: Some(m.into()) } }
}

macro_rules! auth {
    ($headers:expr, $state:expr) => {
        if !check_auth($headers, &$state.api_token) {
            return (StatusCode::UNAUTHORIZED, Json(ErrorResp::new("unauthorized"))).into_response();
        }
        $state.metrics.inc_api_requests();
    };
}

// ─── System ──────────────────────────────────────────────────────────────────

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn system_status(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    let agents = state.registry.list().await;
    let running = agents.iter().filter(|a| a.status == AgentStatus::Running).count();
    Json(serde_json::json!({
        "daemon":  "running",
        "version": env!("CARGO_PKG_VERSION"),
        "agents":  agents.len(),
        "running": running,
        "api":     state.cfg.api.rest_bind,
    })).into_response()
}

async fn system_reload(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    unsafe { libc::kill(std::process::id() as i32, libc::SIGHUP); }
    Json(OkResp::msg("reload signal sent")).into_response()
}

// ─── Agents ──────────────────────────────────────────────────────────────────

async fn list_agents(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    Json(state.registry.list().await).into_response()
}

#[derive(Deserialize)]
pub struct RegisterAgentReq { pub manifest: iad_core::AgentManifest }

async fn register_agent(
    headers:          HeaderMap,
    State(state):     State<AppState>,
    Json(req):        Json<RegisterAgentReq>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.register(req.manifest).await {
        Ok(agent) => {
            let _ = state.storage.upsert_agent(&agent);
            state.metrics.inc_agents_registered();
            let _ = state.event_bus.publish(
                "agent", "agent.registered", "api",
                serde_json::json!({ "name": agent.name, "id": agent.id }),
            ).await;
            (StatusCode::CREATED, Json(serde_json::to_value(&agent).unwrap())).into_response()
        }
        Err(e) => (StatusCode::CONFLICT, Json(ErrorResp::new(&e.to_string()))).into_response(),
    }
}

async fn get_agent(
    headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.get(&name).await {
        Ok(a)  => Json(serde_json::to_value(&a).unwrap()).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn delete_agent(
    headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.deregister(&name).await {
        Ok(_) => {
            let _ = state.storage.delete_agent_by_name(&name);
            let _ = state.event_bus.publish("agent", "agent.removed", "api",
                serde_json::json!({ "name": name })).await;
            Json(OkResp::msg(&format!("agent '{name}' removed"))).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn start_agent(
    headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.update(&name, |a| a.status = AgentStatus::Registered).await {
        Ok(_)  => Json(OkResp::msg(&format!("agent '{name}' queued for start"))).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn stop_agent(
    headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.get(&name).await {
        Ok(_)  => Json(OkResp::msg(&format!("stop queued for '{name}'"))).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn agent_logs(
    headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.storage.list_incidents_for_agent(&name) {
        Ok(i)  => Json(i).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResp::new(&e.to_string()))).into_response(),
    }
}

// ─── Tasks ───────────────────────────────────────────────────────────────────

async fn list_tasks(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    Json(serde_json::json!({ "tasks": [], "note": "Phase 2 task persistence" })).into_response()
}

#[derive(Deserialize)]
pub struct SubmitTaskReq {
    pub payload:      serde_json::Value,
    pub capabilities: Vec<String>,
    pub priority:     Option<u8>,
    pub max_retries:  Option<u32>,
}

async fn submit_task(
    headers: HeaderMap, State(state): State<AppState>, Json(req): Json<SubmitTaskReq>,
) -> axum::response::Response {
    auth!(&headers, state);
    let id = ulid::Ulid::new().to_string();
    state.metrics.inc_tasks_submitted();
    let _ = state.event_bus.publish("task", "task.submitted", "api",
        serde_json::json!({ "task_id": id, "capabilities": req.capabilities })).await;
    Json(serde_json::json!({ "task_id": id, "status": "pending" })).into_response()
}

async fn get_task(
    headers: HeaderMap, State(state): State<AppState>, Path(id): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    (StatusCode::NOT_FOUND, Json(ErrorResp::new("task not found"))).into_response()
}

async fn cancel_task(
    headers: HeaderMap, State(state): State<AppState>, Path(id): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    Json(OkResp::msg(&format!("task '{id}' cancel queued"))).into_response()
}

async fn list_dead_letter(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    Json(serde_json::json!({ "dead_letter": [] })).into_response()
}

async fn retry_dead_letter(
    headers: HeaderMap, State(state): State<AppState>, Path(id): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    Json(OkResp::msg(&format!("task '{id}' retry queued"))).into_response()
}

// ─── API tests ───────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests;
