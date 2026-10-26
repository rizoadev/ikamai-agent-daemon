//! Unix socket IPC server — accepts agent connections.

use crate::{
    ipc::{read_frame, write_frame, AgentFrame, DaemonFrame},
    registry::AgentRegistry,
    AgentStatus, Result,
};
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use std::collections::HashMap;
use tracing::{error, info, warn};
use ulid::Ulid;

/// Per-agent connection handle — send frames to agent.
pub type AgentTx = mpsc::Sender<DaemonFrame>;

/// Task result stored after completion.
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id:    String,
    pub agent_name: String,
    pub result:     Option<serde_json::Value>,
    pub error:      Option<String>,
    pub completed:  bool,
    pub completed_at: chrono::DateTime<Utc>,
}

/// Broadcast channel for task results (any listener can receive).
pub type ResultTx = broadcast::Sender<TaskResult>;
pub type ResultRx = broadcast::Receiver<TaskResult>;

/// Shared state passed into IPC server.
#[derive(Clone)]
pub struct IpcContext {
    pub registry:   Arc<AgentRegistry>,
    pub result_tx:  ResultTx,
    pub metrics_tx: mpsc::Sender<IpcMetricEvent>,
}

/// Metric events emitted by IPC server.
#[derive(Debug)]
pub enum IpcMetricEvent {
    TaskCompleted,
    TaskFailed,
    AgentConnected(String),
    AgentDisconnected(String),
}

/// Registry of connected agent sockets: agent_name → sender.
#[derive(Clone, Default)]
pub struct ConnectionRegistry {
    conns: Arc<Mutex<HashMap<String, AgentTx>>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self { Self::default() }

    pub async fn register(&self, name: &str, tx: AgentTx) {
        self.conns.lock().await.insert(name.into(), tx);
        info!(agent = name, "agent IPC connected");
    }

    pub async fn unregister(&self, name: &str) {
        self.conns.lock().await.remove(name);
        info!(agent = name, "agent IPC disconnected");
    }

    pub async fn send(&self, name: &str, frame: DaemonFrame) -> bool {
        let conns = self.conns.lock().await;
        if let Some(tx) = conns.get(name) {
            tx.send(frame).await.is_ok()
        } else {
            false
        }
    }

    pub async fn connected_agents(&self) -> Vec<String> {
        self.conns.lock().await.keys().cloned().collect()
    }

    /// Broadcast a frame to ALL connected agents.
    pub async fn broadcast(&self, frame: DaemonFrame) -> usize {
        let conns = self.conns.lock().await;
        let mut sent = 0;
        for tx in conns.values() {
            if tx.send(frame.clone()).await.is_ok() { sent += 1; }
        }
        sent
    }

    /// Send to agents matching a team (all agents whose name is in list).
    pub async fn send_team(&self, members: &[String], frame: DaemonFrame) -> usize {
        let conns = self.conns.lock().await;
        let mut sent = 0;
        for name in members {
            if let Some(tx) = conns.get(name) {
                if tx.send(frame.clone()).await.is_ok() { sent += 1; }
            }
        }
        sent
    }
}

/// Create shared result channel.
pub fn make_result_channel() -> (ResultTx, ResultRx) {
    broadcast::channel(256)
}

/// Subscribe to task results.
pub fn subscribe_results(tx: &ResultTx) -> ResultRx {
    tx.subscribe()
}

/// Start the Unix socket IPC server.
pub async fn start_ipc_server(
    socket_path: &str,
    ctx:         IpcContext,
    conns:       ConnectionRegistry,
) -> Result<()> {
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)
        .map_err(|e| crate::Error::Io(e))?;
    info!(path = socket_path, "IPC socket listening");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let ctx2   = ctx.clone();
                let conns2 = conns.clone();
                tokio::spawn(handle_agent_conn(stream, ctx2, conns2));
            }
            Err(e) => error!("IPC accept error: {e}"),
        }
    }
}

/// Handle one agent connection.
async fn handle_agent_conn(
    mut stream: UnixStream,
    ctx:        IpcContext,
    conns:      ConnectionRegistry,
) {
    let first: AgentFrame = match read_frame(&mut stream).await {
        Ok(f)  => f,
        Err(e) => { warn!("IPC first frame error: {e}"); return; }
    };

    let (agent_name, _caps) = match first {
        AgentFrame::Capabilities { caps } => {
            let agents = ctx.registry.list().await;
            let found  = agents.iter().find(|a| {
                caps.iter().any(|c| a.manifest.capabilities.contains(c))
            });
            let name = match found {
                Some(a) => a.name.clone(),
                None    => {
                    warn!(?caps, "no registered agent matches capabilities");
                    caps.first().cloned().unwrap_or_else(|| format!("agent-{}", Ulid::new()))
                }
            };
            (name, caps)
        }
        _ => { warn!("expected Capabilities frame first"); return; }
    };

    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<DaemonFrame>(64);

    conns.register(&agent_name, tx).await;

    // Mark agent running in registry
    let _ = ctx.registry.update(&agent_name, |a| {
        a.status = AgentStatus::Running;
        a.started_at = Some(Utc::now());
    }).await;

    // Emit metric
    let _ = ctx.metrics_tx.send(IpcMetricEvent::AgentConnected(agent_name.clone())).await;

    // Writer task: daemon → agent
    let name_w = agent_name.clone();
    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if let Err(e) = write_frame(&mut writer, &frame).await {
                warn!(agent = name_w, "IPC write error: {e}"); break;
            }
        }
    });

    // Reader loop: agent → daemon
    loop {
        let frame: AgentFrame = match read_frame(&mut reader).await {
            Ok(f)  => f,
            Err(_) => break,
        };

        match frame {
            AgentFrame::Pong { .. } => {}

            AgentFrame::TaskComplete { task_id, result } => {
                info!(agent = agent_name, task_id, "task completed");
                let _ = ctx.metrics_tx.send(IpcMetricEvent::TaskCompleted).await;
                let _ = ctx.result_tx.send(TaskResult {
                    task_id:      task_id.clone(),
                    agent_name:   agent_name.clone(),
                    result:       Some(result.clone()),
                    error:        None,
                    completed:    true,
                    completed_at: Utc::now(),
                });
                info!(agent = agent_name, task_id, ?result, "TASK RESULT");
            }

            AgentFrame::TaskFailed { task_id, error, .. } => {
                warn!(agent = agent_name, task_id, error, "task failed");
                let _ = ctx.metrics_tx.send(IpcMetricEvent::TaskFailed).await;
                let _ = ctx.result_tx.send(TaskResult {
                    task_id:      task_id.clone(),
                    agent_name:   agent_name.clone(),
                    result:       None,
                    error:        Some(error),
                    completed:    false,
                    completed_at: Utc::now(),
                });
            }

            AgentFrame::TaskHeartbeat { task_id, .. } => {
                info!(agent = agent_name, task_id, "task heartbeat");
            }

            AgentFrame::UsageReport { task_id, tokens_in, tokens_out, cost_usd } => {
                info!(agent = agent_name, task_id, tokens_in, tokens_out, cost_usd, "usage");
            }

            AgentFrame::Capabilities { .. } => {}
        }
    }

    // Cleanup
    conns.unregister(&agent_name).await;
    let _ = ctx.registry.update(&agent_name, |a| {
        a.status     = AgentStatus::Stopped;
        a.pid        = None;
        a.stopped_at = Some(Utc::now());
    }).await;
    let _ = ctx.metrics_tx.send(IpcMetricEvent::AgentDisconnected(agent_name)).await;
}
