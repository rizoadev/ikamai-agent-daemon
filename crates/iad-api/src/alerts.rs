//! Alert webhooks (Phase 3.10).

use iad_core::models::{Agent, Task};
use serde::Serialize;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "alert_type", rename_all = "snake_case")]
pub enum AlertPayload {
    AgentCrashed {
        agent_name:    String,
        generation_id: u64,
        failure_count: u32,
        exit_code:     Option<i32>,
        signal:        Option<i32>,
    },
    TaskFailed {
        task_id:     String,
        error:       String,
        retry_count: u32,
        dead_letter: bool,
    },
}

#[derive(Debug, Clone)]
pub struct AlertWebhook {
    pub url:    String,
    pub secret: Option<String>,
}

/// Alert manager — fires HTTP webhooks on critical events.
#[derive(Clone, Default)]
pub struct AlertManager {
    pub webhooks: Vec<AlertWebhook>,
}

impl AlertManager {
    pub fn new(webhooks: Vec<AlertWebhook>) -> Self {
        Self { webhooks }
    }

    pub async fn fire(&self, payload: AlertPayload) {
        if self.webhooks.is_empty() { return; }
        let body = match serde_json::to_string(&payload) {
            Ok(b)  => b,
            Err(e) => { warn!("alert serialize error: {e}"); return; }
        };
        for hook in &self.webhooks {
            // Use tokio::process to curl (avoids reqwest dep in base build).
            let status = tokio::process::Command::new("curl")
                .args(["-s", "-X", "POST",
                       "-H", "Content-Type: application/json",
                       "-d", &body,
                       &hook.url])
                .status()
                .await;
            match status {
                Ok(s)  => info!(url = hook.url, code = s.code(), "alert sent"),
                Err(e) => warn!(url = hook.url, "alert failed: {e}"),
            }
        }
    }

    pub async fn agent_crashed(&self, agent: &Agent, exit_code: Option<i32>, signal: Option<i32>) {
        self.fire(AlertPayload::AgentCrashed {
            agent_name:    agent.name.clone(),
            generation_id: agent.generation_id,
            failure_count: agent.failure_count,
            exit_code,
            signal,
        }).await;
    }

    pub async fn task_failed(&self, task: &Task, error: &str, dead_letter: bool) {
        self.fire(AlertPayload::TaskFailed {
            task_id:     task.id.clone(),
            error:       error.into(),
            retry_count: task.retry_count,
            dead_letter,
        }).await;
    }
}
