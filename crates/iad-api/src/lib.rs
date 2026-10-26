//! IKAMAI Agent Daemon — REST API server.

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
use iad_core::{
    AgentRegistry, AgentStatus, DaemonConfig,
    event_bus::EventBus,
    ipc::DaemonFrame,
    ipc_server::ConnectionRegistry,
};
use iad_storage::Storage;
use metrics::Metrics;
use alerts::AlertManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

// ─── App State ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub storage:       Storage,
    pub registry:      Arc<AgentRegistry>,
    pub cfg:           DaemonConfig,
    pub api_token:     Option<String>,
    pub metrics:       Arc<Metrics>,
    pub event_bus:     Arc<EventBus>,
    pub alert_mgr:     Option<Arc<AlertManager>>,
    pub conn_registry: Option<ConnectionRegistry>,
}

// ─── Router ───────────────────────────────────────────────────────────────────

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz",                              get(healthz))
        .route("/metrics",                              get(metrics::metrics_handler))
        .route("/api/v1/status",                        get(system_status))
        .route("/api/v1/reload",                        post(system_reload))
        .route("/api/v1/ws",                            get(ws::ws_handler))
        // Agents
        .route("/api/v1/agents",                        get(list_agents).post(register_agent))
        .route("/api/v1/agents/:name",                  get(get_agent).delete(delete_agent))
        .route("/api/v1/agents/:name/start",            post(start_agent))
        .route("/api/v1/agents/:name/stop",             post(stop_agent))
        .route("/api/v1/agents/:name/logs",             get(agent_logs))
        // Tasks
        .route("/api/v1/tasks",                         get(list_tasks).post(submit_task))
        .route("/api/v1/tasks/dispatch",                post(dispatch_task))
        .route("/api/v1/tasks/await/:task_id",          get(await_task_result))
        .route("/api/v1/tasks/dead-letter",             get(list_dead_letter))
        .route("/api/v1/tasks/dead-letter/:id/retry",   post(retry_dead_letter))
        .route("/api/v1/tasks/:id",                     get(get_task).delete(cancel_task))
        // Teams
        .route("/api/v1/teams",                         get(list_teams))
        .route("/api/v1/teams/:team/mention",           post(mention_team))
        .route("/api/v1/teams/:team/connected",         get(team_connected))
        .with_state(state)
}

// ─── Auth ─────────────────────────────────────────────────────────────────────

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

// ─── Responses ────────────────────────────────────────────────────────────────

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

// ─── System ───────────────────────────────────────────────────────────────────

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn system_status(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    let agents  = state.registry.list().await;
    let running = agents.iter().filter(|a| a.status == AgentStatus::Running).count();
    let connected = state.conn_registry.as_ref()
        .map(|cr| {
            // We need async but macro forces sync — spawn and return count from registry
            0usize // placeholder; real count via /api/v1/agents
        })
        .unwrap_or(0);
    Json(serde_json::json!({
        "daemon":    "running",
        "version":   env!("CARGO_PKG_VERSION"),
        "agents":    agents.len(),
        "running":   running,
        "api":       state.cfg.api.rest_bind,
    })).into_response()
}

async fn system_reload(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    unsafe { libc::kill(std::process::id() as i32, libc::SIGHUP); }
    Json(OkResp::msg("reload signal sent")).into_response()
}

// ─── Agents ───────────────────────────────────────────────────────────────────

async fn list_agents(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    let agents = state.registry.list().await;
    Json(agents).into_response()
}

#[derive(Deserialize)]
pub struct RegisterAgentReq { pub manifest: iad_core::AgentManifest }

async fn register_agent(
    headers: HeaderMap, State(state): State<AppState>, Json(req): Json<RegisterAgentReq>,
) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.register(req.manifest).await {
        Ok(agent) => {
            let _ = state.storage.upsert_agent(&agent);
            state.metrics.inc_agents_registered();
            let _ = state.event_bus.publish("agent", "agent.registered", "api",
                serde_json::json!({ "name": agent.name })).await;
            (StatusCode::CREATED, Json(serde_json::to_value(&agent).unwrap())).into_response()
        }
        Err(e) => (StatusCode::CONFLICT, Json(ErrorResp::new(&e.to_string()))).into_response(),
    }
}

async fn get_agent(headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.get(&name).await {
        Ok(a)  => Json(serde_json::to_value(&a).unwrap()).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn delete_agent(headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.deregister(&name).await {
        Ok(_) => {
            let _ = state.storage.delete_agent_by_name(&name);
            Json(OkResp::msg(&format!("agent '{name}' removed"))).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn start_agent(headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    match state.registry.update(&name, |a| a.status = AgentStatus::Registered).await {
        Ok(_)  => Json(OkResp::msg(&format!("'{name}' queued for start"))).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(ErrorResp::new("agent not found"))).into_response(),
    }
}

async fn stop_agent(headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    // Send SHUTDOWN frame to connected agent
    if let Some(cr) = &state.conn_registry {
        cr.send(&name, DaemonFrame::Shutdown).await;
    }
    Json(OkResp::msg(&format!("shutdown sent to '{name}'"))).into_response()
}

async fn agent_logs(headers: HeaderMap, State(state): State<AppState>, Path(name): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    match state.storage.list_incidents_for_agent(&name) {
        Ok(i)  => Json(i).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResp::new(&e.to_string()))).into_response(),
    }
}

// ─── Tasks ────────────────────────────────────────────────────────────────────

async fn list_tasks(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    Json(serde_json::json!({ "tasks": [], "note": "Phase 2 persistence" })).into_response()
}

#[derive(Deserialize)]
pub struct SubmitTaskReq {
    pub payload:      serde_json::Value,
    pub capabilities: Vec<String>,
    pub priority:     Option<u8>,
    pub max_retries:  Option<u32>,
}

async fn submit_task(headers: HeaderMap, State(state): State<AppState>, Json(req): Json<SubmitTaskReq>) -> axum::response::Response {
    auth!(&headers, state);
    let id = ulid::Ulid::new().to_string();
    state.metrics.inc_tasks_submitted();
    Json(serde_json::json!({ "task_id": id, "status": "pending" })).into_response()
}

// ─── Dispatch (send to specific connected agent, wait for result) ─────────────

#[derive(Deserialize)]
pub struct DispatchReq {
    pub agent_name: String,
    pub payload:    serde_json::Value,
    /// Wait up to N seconds for result (default 30)
    pub timeout_secs: Option<u64>,
}

async fn dispatch_task(
    headers: HeaderMap, State(state): State<AppState>, Json(req): Json<DispatchReq>,
) -> axum::response::Response {
    auth!(&headers, state);
    let task_id = ulid::Ulid::new().to_string();

    let Some(cr) = &state.conn_registry else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResp::new("IPC not available"))).into_response();
    };

    let frame = DaemonFrame::AssignTask { task_id: task_id.clone(), payload: req.payload.clone() };
    if !cr.send(&req.agent_name, frame).await {
        return (StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResp::new(&format!("agent '{}' not connected", req.agent_name)))).into_response();
    }

    state.metrics.inc_tasks_submitted();
    let _ = state.event_bus.publish("task", "task.dispatched", "api",
        serde_json::json!({ "task_id": task_id, "agent": req.agent_name })).await;

    Json(serde_json::json!({
        "task_id": task_id,
        "status":  "dispatched",
        "agent":   req.agent_name,
    })).into_response()
}

/// Long-poll: wait for task result (SSE-style via result_rx is Phase 3+ — here we use stored results)
async fn await_task_result(
    headers: HeaderMap, State(state): State<AppState>, Path(task_id): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    // Phase 2 stub — result available via WebSocket /api/v1/ws
    Json(serde_json::json!({
        "task_id": task_id,
        "note": "Subscribe to ws://127.0.0.1:7700/api/v1/ws?topics=task for live results"
    })).into_response()
}

async fn get_task(headers: HeaderMap, State(state): State<AppState>, Path(id): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    (StatusCode::NOT_FOUND, Json(ErrorResp::new("task not found"))).into_response()
}

async fn cancel_task(headers: HeaderMap, State(state): State<AppState>, Path(id): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    if let Some(cr) = &state.conn_registry {
        cr.send(&id, DaemonFrame::CancelTask { task_id: id.clone() }).await;
    }
    Json(OkResp::msg(&format!("cancel sent for task '{id}'"))).into_response()
}

async fn list_dead_letter(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    Json(serde_json::json!({ "dead_letter": [] })).into_response()
}

async fn retry_dead_letter(headers: HeaderMap, State(state): State<AppState>, Path(id): Path<String>) -> axum::response::Response {
    auth!(&headers, state);
    Json(OkResp::msg(&format!("task '{id}' retry queued"))).into_response()
}

// ─── Teams ────────────────────────────────────────────────────────────────────

/// Team config — hardcoded for now, future: load from DB/file
fn get_teams() -> std::collections::HashMap<String, Vec<String>> {
    let mut m = std::collections::HashMap::new();
    m.insert("web-team".into(),  vec!["designer-agent".into(), "coder-agent".into()]);
    m.insert("all".into(),       vec!["designer-agent".into(), "coder-agent".into(),
                                      "reviewer-agent".into(), "landingpage-agent".into()]);
    m
}

async fn list_teams(headers: HeaderMap, State(state): State<AppState>) -> axum::response::Response {
    auth!(&headers, state);
    let teams = get_teams();
    Json(serde_json::json!({ "teams": teams })).into_response()
}

#[derive(Deserialize)]
pub struct MentionReq {
    /// The message / task payload sent to all agents in the team.
    pub message:  String,
    pub payload:  Option<serde_json::Value>,
    /// Optionally route through leader first.
    pub leader:   Option<String>,
}

/// @team mention — broadcast task to all agents in a team.
async fn mention_team(
    headers: HeaderMap, State(state): State<AppState>,
    Path(team): Path<String>, Json(req): Json<MentionReq>,
) -> axum::response::Response {
    auth!(&headers, state);

    let teams = get_teams();
    let members = match teams.get(&team) {
        Some(m) => m.clone(),
        None    => {
            return (StatusCode::NOT_FOUND, Json(ErrorResp::new(&format!("team '{team}' not found")))).into_response();
        }
    };

    let Some(cr) = &state.conn_registry else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResp::new("IPC not available"))).into_response();
    };

    let base_payload = req.payload.clone().unwrap_or_else(|| serde_json::json!({ "message": req.message }));

    // If leader set: route through leader, leader will fan out.
    // Otherwise: broadcast directly to all team members.
    let mut dispatched = vec![];
    let mut failed     = vec![];

    if let Some(leader) = &req.leader {
        let task_id = ulid::Ulid::new().to_string();
        let payload = serde_json::json!({
            "type":     "team_task",
            "message":  req.message,
            "team":     team,
            "members":  members,
            "payload":  base_payload,
        });
        let frame = DaemonFrame::AssignTask { task_id: task_id.clone(), payload };
        if cr.send(leader, frame).await {
            dispatched.push(leader.clone());
        } else {
            failed.push(leader.clone());
        }
    } else {
        for member in &members {
            let task_id = ulid::Ulid::new().to_string();
            let payload = serde_json::json!({
                "message": req.message,
                "team":    team,
                "payload": base_payload,
            });
            let frame = DaemonFrame::AssignTask { task_id: task_id.clone(), payload };
            if cr.send(member, frame).await {
                dispatched.push(member.clone());
                state.metrics.inc_tasks_submitted();
            } else {
                failed.push(member.clone());
            }
        }
    }

    let _ = state.event_bus.publish("team", "team.mentioned", "api",
        serde_json::json!({ "team": team, "dispatched": dispatched, "message": req.message })).await;

    Json(serde_json::json!({
        "team":       team,
        "dispatched": dispatched,
        "not_connected": failed,
        "message":    req.message,
    })).into_response()
}

async fn team_connected(
    headers: HeaderMap, State(state): State<AppState>, Path(team): Path<String>,
) -> axum::response::Response {
    auth!(&headers, state);
    let teams = get_teams();
    let members = teams.get(&team).cloned().unwrap_or_default();
    let connected = match &state.conn_registry {
        Some(cr) => {
            let all = cr.connected_agents().await;
            members.iter().filter(|m| all.contains(m)).cloned().collect::<Vec<_>>()
        }
        None => vec![],
    };
    Json(serde_json::json!({
        "team":      team,
        "members":   members,
        "connected": connected,
    })).into_response()
}

// ─── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests;
