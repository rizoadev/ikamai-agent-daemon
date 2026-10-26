//! Domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Agent status in the daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Registered,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Disabled,
}

/// Agent manifest — defines an agent's properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub version: String,
    pub runtime: String,
    pub entrypoint: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// Agent record in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: AgentStatus,
    pub pid: Option<u32>,
    pub manifest: AgentManifest,
    pub failure_count: u32,
    pub started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Task priority levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    None = 0,
    Urgent = 1,
    High = 2,
    Medium = 3,
    Low = 4,
}

/// Task status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Queued,
    Assigned,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: Option<String>,
    pub payload: serde_json::Value,
    pub capabilities: Vec<String>,
    pub priority: Priority,
    pub status: TaskStatus,
    pub assigned_agent: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub topic: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub source: String,
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
}
