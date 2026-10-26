//! Domain models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Agent ───────────────────────────────────────────────────────────────────

/// Agent lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Registered,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Crashed,
    Disabled,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Registered => "registered",
            Self::Starting   => "starting",
            Self::Running    => "running",
            Self::Stopping   => "stopping",
            Self::Stopped    => "stopped",
            Self::Failed     => "failed",
            Self::Crashed    => "crashed",
            Self::Disabled   => "disabled",
        };
        write!(f, "{s}")
    }
}

/// Restart policy for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Never restart.
    Never,
    /// Restart only on failure.
    OnFailure,
    /// Always restart.
    Always,
}

impl Default for RestartPolicy {
    fn default() -> Self { Self::OnFailure }
}

/// Agent manifest — YAML file that describes an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name:        String,
    pub version:     String,
    pub runtime:     String,
    pub entrypoint:  String,
    #[serde(default)]
    pub args:        Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub env:         std::collections::HashMap<String, String>,
    #[serde(default)]
    pub restart:     RestartPolicy,
    /// Max restart attempts before giving up (0 = unlimited).
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
}

fn default_max_restarts() -> u32 { 5 }

/// Agent record stored in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id:            String,
    pub name:          String,
    pub version:       String,
    pub status:        AgentStatus,
    pub pid:           Option<u32>,
    pub manifest:      AgentManifest,
    /// Monotonically increasing; incremented on each crash/restart.
    pub generation_id: u64,
    pub failure_count: u32,
    pub started_at:    Option<DateTime<Utc>>,
    pub stopped_at:    Option<DateTime<Utc>>,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

// ─── Task ────────────────────────────────────────────────────────────────────

/// Task priority levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Priority {
    Urgent = 1,
    High   = 2,
    Medium = 3,
    Low    = 4,
    None   = 0,
}

impl Default for Priority {
    fn default() -> Self { Self::Medium }
}

/// Task lifecycle status.
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
    DeadLetter,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending    => "pending",
            Self::Queued     => "queued",
            Self::Assigned   => "assigned",
            Self::Running    => "running",
            Self::Completed  => "completed",
            Self::Failed     => "failed",
            Self::Cancelled  => "cancelled",
            Self::DeadLetter => "dead_letter",
        };
        write!(f, "{s}")
    }
}

/// A task record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id:             String,
    pub title:          Option<String>,
    pub payload:        serde_json::Value,
    pub capabilities:   Vec<String>,
    pub priority:       Priority,
    pub status:         TaskStatus,
    pub assigned_agent: Option<String>,
    pub result:         Option<serde_json::Value>,
    pub error:          Option<String>,
    pub retry_count:    u32,
    pub max_retries:    u32,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
}

// ─── Event ───────────────────────────────────────────────────────────────────

/// An event on the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id:         String,
    pub topic:      String,
    pub event_type: String,
    pub payload:    serde_json::Value,
    pub source:     String,
    pub sequence:   u64,
    pub created_at: DateTime<Utc>,
}

// ─── Agent Incident ──────────────────────────────────────────────────────────

/// Crash / incident record (flight recorder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIncident {
    pub id:           String,
    pub agent_id:     String,
    pub agent_name:   String,
    pub generation_id: u64,
    pub exit_code:    Option<i32>,
    pub signal:       Option<i32>,
    pub reason:       String,
    pub occurred_at:  DateTime<Utc>,
}
