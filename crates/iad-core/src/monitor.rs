//! Real-time agent monitor — aggregates streaming events into live state.
//!
//! The Monitor tracks per-agent phase (idle → thinking → tool → writing → completed),
//! buffers thinking text, tool outputs, and text deltas, and provides
//! a dashboard snapshot on demand.

use crate::models::{
    AgentMonitorState, AgentPhase, AgentStatus, DashboardSnapshot,
    ThinkingState, ToolCallState,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Central monitor that holds live state for every agent.
#[derive(Clone)]
pub struct Monitor {
    agents:    Arc<RwLock<HashMap<String, AgentMonitorState>>>,
    daemon_start: chrono::DateTime<chrono::Utc>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            agents:       Arc::new(RwLock::new(HashMap::new())),
            daemon_start: Utc::now(),
        }
    }

    // ─── Agent lifecycle ─────────────────────────────────────────────────

    pub async fn agent_connected(&self, name: &str) {
        let mut map = self.agents.write().await;
        let entry = map.entry(name.into()).or_insert_with(|| AgentMonitorState {
            agent_name:    name.into(),
            status:        AgentStatus::Running,
            current_task:  None,
            phase:         AgentPhase::Idle,
            thinking:      None,
            tool_call:     None,
            text_buffer:   String::new(),
            tasks_done:    0,
            tasks_failed:  0,
            uptime_secs:   0,
            cpu_pct:       0.0,
            mem_bytes:     0,
            last_activity: Utc::now(),
        });
        entry.status = AgentStatus::Running;
        entry.last_activity = Utc::now();
        info!(agent = name, "monitor: agent connected");
    }

    pub async fn agent_disconnected(&self, name: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(name) {
            state.status = AgentStatus::Stopped;
            state.phase  = AgentPhase::Idle;
            state.current_task = None;
            state.thinking     = None;
            state.tool_call    = None;
            state.last_activity = Utc::now();
        }
        info!(agent = name, "monitor: agent disconnected");
    }

    // ─── Task lifecycle ──────────────────────────────────────────────────

    pub async fn task_assigned(&self, agent: &str, task_id: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.current_task = Some(task_id.into());
            state.phase        = AgentPhase::Idle;
            state.thinking     = None;
            state.tool_call    = None;
            state.text_buffer.clear();
            state.last_activity = Utc::now();
        }
    }

    pub async fn task_completed(&self, agent: &str, _task_id: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.phase = AgentPhase::Completed;
            state.tasks_done += 1;
            state.current_task = None;
            state.thinking     = None;
            state.tool_call    = None;
            state.last_activity = Utc::now();
        }
    }

    pub async fn task_failed(&self, agent: &str, _task_id: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.phase = AgentPhase::Idle;
            state.tasks_failed += 1;
            state.current_task = None;
            state.thinking     = None;
            state.tool_call    = None;
            state.last_activity = Utc::now();
        }
    }

    // ─── Thinking ────────────────────────────────────────────────────────

    pub async fn thinking_start(&self, agent: &str, task_id: &str, content_index: u32) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.phase = AgentPhase::Thinking;
            state.current_task = Some(task_id.into());
            state.thinking = Some(ThinkingState {
                content_index,
                text:        String::new(),
                started_at:  Utc::now(),
                duration_ms: None,
                is_streaming: true,
            });
            state.last_activity = Utc::now();
            debug!(agent, task_id, "monitor: thinking started");
        }
    }

    pub async fn thinking_delta(&self, agent: &str, _task_id: &str, delta: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            if let Some(ref mut t) = state.thinking {
                t.text.push_str(delta);
            }
            state.last_activity = Utc::now();
        }
    }

    pub async fn thinking_end(&self, agent: &str, _task_id: &str, duration_ms: u64) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            if let Some(ref mut t) = state.thinking {
                t.is_streaming = false;
                t.duration_ms = Some(duration_ms);
            }
            // Don't change phase yet — next might be tool or text
            state.last_activity = Utc::now();
            debug!(agent, duration_ms, "monitor: thinking ended");
        }
    }

    // ─── Tool calls ──────────────────────────────────────────────────────

    pub async fn tool_call_start(
        &self, agent: &str, task_id: &str,
        tool_call_id: &str, tool_name: &str, args: &serde_json::Value,
    ) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.phase = AgentPhase::ToolExecution;
            state.current_task = Some(task_id.into());
            state.tool_call = Some(ToolCallState {
                tool_call_id: tool_call_id.into(),
                tool_name:    tool_name.into(),
                args:         args.clone(),
                output:       String::new(),
                started_at:   Utc::now(),
                duration_ms:  None,
                success:      None,
                is_running:   true,
            });
            state.last_activity = Utc::now();
            debug!(agent, tool_name, "monitor: tool call started");
        }
    }

    pub async fn tool_call_update(&self, agent: &str, _task_id: &str, output_delta: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            if let Some(ref mut tc) = state.tool_call {
                tc.output.push_str(output_delta);
            }
            state.last_activity = Utc::now();
        }
    }

    pub async fn tool_call_end(
        &self, agent: &str, _task_id: &str,
        success: bool, duration_ms: u64,
    ) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            if let Some(ref mut tc) = state.tool_call {
                tc.is_running  = false;
                tc.success     = Some(success);
                tc.duration_ms = Some(duration_ms);
            }
            state.last_activity = Utc::now();
            debug!(agent, success, "monitor: tool call ended");
        }
    }

    // ─── Text output ─────────────────────────────────────────────────────

    pub async fn text_delta(&self, agent: &str, _task_id: &str, delta: &str) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.phase = AgentPhase::Writing;
            state.text_buffer.push_str(delta);
            state.last_activity = Utc::now();
        }
    }

    // ─── Task progress ───────────────────────────────────────────────────

    pub async fn task_progress(&self, agent: &str, task_id: &str, step: &str, progress: f32) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.current_task = Some(task_id.into());
            state.last_activity = Utc::now();
            debug!(agent, step, progress, "monitor: progress update");
        }
    }

    // ─── Resource metrics ────────────────────────────────────────────────

    pub async fn resource_metrics(&self, agent: &str, cpu_pct: f32, mem_bytes: u64) {
        let mut map = self.agents.write().await;
        if let Some(state) = map.get_mut(agent) {
            state.cpu_pct   = cpu_pct;
            state.mem_bytes = mem_bytes;
            state.last_activity = Utc::now();
        }
    }

    // ─── Queries ─────────────────────────────────────────────────────────

    /// Get live state for one agent.
    pub async fn get_agent(&self, name: &str) -> Option<AgentMonitorState> {
        let map = self.agents.read().await;
        map.get(name).cloned().map(|mut s| {
            if s.status == AgentStatus::Running {
                if let Some(ref _started) = s.last_activity.checked_sub_signed(chrono::Duration::zero()) {
                    // Calculate uptime from daemon perspective
                    s.uptime_secs = (Utc::now() - self.daemon_start).num_seconds().max(0) as u64;
                }
            }
            s
        })
    }

    /// Full dashboard snapshot.
    pub async fn dashboard(&self) -> DashboardSnapshot {
        let map = self.agents.read().await;
        let agents: Vec<AgentMonitorState> = map.values().cloned().collect();

        let running  = agents.iter().filter(|a| a.status == AgentStatus::Running).count();
        let idle     = agents.iter().filter(|a| a.phase == AgentPhase::Idle && a.status == AgentStatus::Running).count();
        let thinking = agents.iter().filter(|a| a.phase == AgentPhase::Thinking).count();
        let executing= agents.iter().filter(|a| a.phase == AgentPhase::ToolExecution).count();
        let done: u64  = agents.iter().map(|a| a.tasks_done).sum();
        let failed: u64 = agents.iter().map(|a| a.tasks_failed).sum();

        DashboardSnapshot {
            total_agents:       agents.len(),
            agents_running:     running,
            agents_idle:        idle,
            agents_thinking:    thinking,
            agents_executing:   executing,
            total_tasks_done:   done,
            total_tasks_failed: failed,
            uptime_secs:        (Utc::now() - self.daemon_start).num_seconds().max(0) as u64,
            snapshot_at:        Utc::now(),
            agents,
        }
    }

    /// List just agent names and their phases (lightweight poll).
    pub async fn agent_phases(&self) -> Vec<(String, AgentPhase, Option<String>)> {
        let map = self.agents.read().await;
        map.values()
            .map(|a| (a.agent_name.clone(), a.phase.clone(), a.current_task.clone()))
            .collect()
    }
}

impl Default for Monitor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_lifecycle() {
        let mon = Monitor::new();

        // Connect agent
        mon.agent_connected("agent-a").await;
        let snap = mon.dashboard().await;
        assert_eq!(snap.total_agents, 1);
        assert_eq!(snap.agents_running, 1);

        // Assign task
        mon.task_assigned("agent-a", "task-1").await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.current_task, Some("task-1".into()));

        // Thinking
        mon.thinking_start("agent-a", "task-1", 0).await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.phase, AgentPhase::Thinking);
        assert!(a.thinking.as_ref().unwrap().is_streaming);

        mon.thinking_delta("agent-a", "task-1", "Let me analyze ").await;
        mon.thinking_delta("agent-a", "task-1", "this problem...").await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.thinking.as_ref().unwrap().text, "Let me analyze this problem...");

        mon.thinking_end("agent-a", "task-1", 3200).await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.thinking.as_ref().unwrap().duration_ms, Some(3200));
        assert!(!a.thinking.as_ref().unwrap().is_streaming);

        // Tool call
        mon.tool_call_start("agent-a", "task-1", "tc-1", "bash", &serde_json::json!({"cmd": "ls"})).await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.phase, AgentPhase::ToolExecution);
        assert_eq!(a.tool_call.as_ref().unwrap().tool_name, "bash");

        mon.tool_call_update("agent-a", "task-1", "file1.rs\nfile2.rs\n").await;
        mon.tool_call_end("agent-a", "task-1", true, 150).await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert!(!a.tool_call.as_ref().unwrap().is_running);

        // Text output
        mon.text_delta("agent-a", "task-1", "Here is the result: ").await;
        mon.text_delta("agent-a", "task-1", "everything works.").await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.phase, AgentPhase::Writing);
        assert_eq!(a.text_buffer, "Here is the result: everything works.");

        // Complete
        mon.task_completed("agent-a", "task-1").await;
        let a = mon.get_agent("agent-a").await.unwrap();
        assert_eq!(a.tasks_done, 1);
        assert_eq!(a.phase, AgentPhase::Completed);

        // Disconnect
        mon.agent_disconnected("agent-a").await;
        let snap = mon.dashboard().await;
        assert_eq!(snap.agents_running, 0);
    }

    #[tokio::test]
    async fn test_multiple_agents_dashboard() {
        let mon = Monitor::new();
        mon.agent_connected("designer").await;
        mon.agent_connected("coder").await;
        mon.agent_connected("reviewer").await;

        mon.thinking_start("designer", "t1", 0).await;
        mon.tool_call_start("coder", "t2", "tc1", "bash", &serde_json::json!({})).await;

        let snap = mon.dashboard().await;
        assert_eq!(snap.total_agents, 3);
        assert_eq!(snap.agents_running, 3);
        assert_eq!(snap.agents_thinking, 1);
        assert_eq!(snap.agents_executing, 1);
        assert_eq!(snap.agents_idle, 1);
    }

    #[tokio::test]
    async fn test_resource_metrics() {
        let mon = Monitor::new();
        mon.agent_connected("worker").await;
        mon.resource_metrics("worker", 45.2, 1024 * 1024 * 512).await;
        let a = mon.get_agent("worker").await.unwrap();
        assert!((a.cpu_pct - 45.2).abs() < 0.01);
        assert_eq!(a.mem_bytes, 1024 * 1024 * 512);
    }
}
