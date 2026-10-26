//! Prometheus metrics endpoint (Phase 3.9).

use axum::{extract::State, response::Response};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::AppState;

pub struct Metrics {
    pub tasks_submitted:   AtomicU64,
    pub tasks_completed:   AtomicU64,
    pub tasks_failed:      AtomicU64,
    pub tasks_dead_letter: AtomicU64,
    pub agents_registered: AtomicU64,
    pub agents_running:    AtomicU64,
    pub agents_crashed:    AtomicU64,
    pub events_published:  AtomicU64,
    pub ws_connections:    AtomicU64,
    pub api_requests:      AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tasks_submitted:   AtomicU64::new(0),
            tasks_completed:   AtomicU64::new(0),
            tasks_failed:      AtomicU64::new(0),
            tasks_dead_letter: AtomicU64::new(0),
            agents_registered: AtomicU64::new(0),
            agents_running:    AtomicU64::new(0),
            agents_crashed:    AtomicU64::new(0),
            events_published:  AtomicU64::new(0),
            ws_connections:    AtomicU64::new(0),
            api_requests:      AtomicU64::new(0),
        })
    }

    pub fn inc_tasks_submitted(&self)       { self.tasks_submitted.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_tasks_completed(&self)       { self.tasks_completed.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_tasks_failed(&self)          { self.tasks_failed.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_tasks_dead_letter(&self)     { self.tasks_dead_letter.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_agents_registered(&self)     { self.agents_registered.fetch_add(1, Ordering::Relaxed); }
    pub fn set_agents_running(&self, n: u64){ self.agents_running.store(n, Ordering::Relaxed); }
    pub fn inc_agents_crashed(&self)        { self.agents_crashed.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_events_published(&self)      { self.events_published.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_ws_connections(&self)        { self.ws_connections.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_api_requests(&self)          { self.api_requests.fetch_add(1, Ordering::Relaxed); }

    pub fn render(&self, running: u64) -> String {
        self.set_agents_running(running);
        format!(
"# HELP iad_tasks_submitted_total Total tasks submitted
# TYPE iad_tasks_submitted_total counter
iad_tasks_submitted_total {ts}

# HELP iad_tasks_completed_total Total tasks completed
# TYPE iad_tasks_completed_total counter
iad_tasks_completed_total {tc}

# HELP iad_tasks_failed_total Total tasks failed
# TYPE iad_tasks_failed_total counter
iad_tasks_failed_total {tf}

# HELP iad_tasks_dead_letter_total Tasks in dead-letter queue
# TYPE iad_tasks_dead_letter_total gauge
iad_tasks_dead_letter_total {tdl}

# HELP iad_agents_registered_total Agents ever registered
# TYPE iad_agents_registered_total counter
iad_agents_registered_total {ar}

# HELP iad_agents_running Current running agents
# TYPE iad_agents_running gauge
iad_agents_running {run}

# HELP iad_agents_crashed_total Agents crashed
# TYPE iad_agents_crashed_total counter
iad_agents_crashed_total {ac}

# HELP iad_events_published_total Events published on bus
# TYPE iad_events_published_total counter
iad_events_published_total {ep}

# HELP iad_websocket_connections_total WebSocket connections
# TYPE iad_websocket_connections_total counter
iad_websocket_connections_total {wsc}

# HELP iad_api_requests_total API requests handled
# TYPE iad_api_requests_total counter
iad_api_requests_total {req}
",
            ts  = self.tasks_submitted.load(Ordering::Relaxed),
            tc  = self.tasks_completed.load(Ordering::Relaxed),
            tf  = self.tasks_failed.load(Ordering::Relaxed),
            tdl = self.tasks_dead_letter.load(Ordering::Relaxed),
            ar  = self.agents_registered.load(Ordering::Relaxed),
            run = running,
            ac  = self.agents_crashed.load(Ordering::Relaxed),
            ep  = self.events_published.load(Ordering::Relaxed),
            wsc = self.ws_connections.load(Ordering::Relaxed),
            req = self.api_requests.load(Ordering::Relaxed),
        )
    }
}

pub async fn metrics_handler(State(state): State<AppState>) -> Response {
    let agents  = state.registry.list().await;
    let running = agents.iter().filter(|a| a.status == iad_core::AgentStatus::Running).count() as u64;
    let body    = state.metrics.render(running);
    Response::builder()
        .status(200)
        .header(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(axum::body::Body::from(body))
        .unwrap()
}
