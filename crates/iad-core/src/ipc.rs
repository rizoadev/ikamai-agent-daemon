//! Agent-daemon IPC protocol over Unix sockets.
//!
//! Frame format (length-prefixed JSON):
//!   [u32 LE: payload_len][payload_bytes: UTF-8 JSON]
//!
//! Implements: task 2.1 — frame timeouts, max size

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Max frame size: 4 MiB
pub const MAX_FRAME_SIZE: u32 = 4 * 1024 * 1024;

/// Frames sent from daemon → agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DaemonFrame {
    /// Assign a task.
    AssignTask { task_id: String, payload: serde_json::Value },
    /// Cancel an in-flight task.
    CancelTask { task_id: String },
    /// Keepalive ping.
    Ping { seq: u64 },
    /// Graceful shutdown.
    Shutdown,
}

/// Frames sent from agent → daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentFrame {
    /// Agent is alive.
    Pong { seq: u64 },
    /// Task completed successfully.
    TaskComplete { task_id: String, result: serde_json::Value },
    /// Task failed.
    TaskFailed {
        task_id:     String,
        error:       String,
        error_class: ErrorClass,
        retryable:   bool,
    },
    /// Task heartbeat (lease renewal).
    TaskHeartbeat { task_id: String, lease_token: String },
    /// Usage reporting.
    UsageReport {
        task_id:    String,
        tokens_in:  u64,
        tokens_out: u64,
        cost_usd:   f64,
    },
    /// Agent capabilities advertisement.
    Capabilities { caps: Vec<String> },

    // ─── Real-time Streaming Frames (Phase 3.10) ───────────────────────

    /// Agent started thinking / reasoning.
    ThinkingStart {
        task_id:       String,
        content_index: u32,
    },
    /// Incremental thinking text delta.
    ThinkingDelta {
        task_id:       String,
        content_index: u32,
        delta:         String,
    },
    /// Agent finished thinking.
    ThinkingEnd {
        task_id:       String,
        content_index: u32,
        duration_ms:   u64,
    },
    /// Agent started a tool call.
    ToolCallStart {
        task_id:     String,
        tool_call_id: String,
        tool_name:   String,
        args:        serde_json::Value,
    },
    /// Tool execution progress update.
    ToolCallUpdate {
        task_id:     String,
        tool_call_id: String,
        output_delta: String,
    },
    /// Tool call finished.
    ToolCallEnd {
        task_id:      String,
        tool_call_id: String,
        result:       serde_json::Value,
        success:      bool,
        duration_ms:  u64,
    },
    /// Streaming text response delta.
    TextDelta {
        task_id:       String,
        content_index: u32,
        delta:         String,
    },
    /// Task progress percentage / step update.
    TaskProgress {
        task_id:  String,
        step:     String,
        progress: f32,
        detail:   Option<String>,
    },
    /// Agent resource metrics snapshot.
    ResourceMetrics {
        cpu_pct:      f32,
        mem_bytes:    u64,
        active_tasks: u32,
    },
}

/// Error classification (plan D10).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    /// Transient — safe to retry.
    Transient,
    /// Rate limit — retry after delay.
    RateLimit,
    /// Fatal — do not retry.
    Fatal,
    /// Timeout — may retry.
    Timeout,
}

impl ErrorClass {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient | Self::RateLimit | Self::Timeout)
    }
}

/// Write a framed JSON message to a writer.
pub async fn write_frame<W, T>(writer: &mut W, msg: &T) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_vec(msg).map_err(|e| Error::Serde(e.to_string()))?;
    if payload.len() > MAX_FRAME_SIZE as usize {
        return Err(Error::Process(format!("frame too large: {} bytes", payload.len())));
    }
    let len = payload.len() as u32;
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&payload).await?;
    Ok(())
}

/// Read a framed JSON message from a reader.
pub async fn read_frame<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncReadExt + Unpin,
    T: for<'de> Deserialize<'de>,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME_SIZE {
        return Err(Error::Process(format!("frame too large: {len} bytes")));
    }
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf).map_err(|e| Error::Serde(e.to_string()))
}
