//! Process manager — spawn, kill, monitor agent processes.
//!
//! Implements:
//! - Process groups (plan D7, task 1.R2)
//! - PR_SET_PDEATHSIG / CLOEXEC (task 1.R3)
//! - SIGCHLD / zombie reaping (task 1.R1)
//! - Exponential backoff restarts (task 1.9)

use crate::{
    models::{Agent, AgentIncident, AgentManifest, RestartPolicy},
    Error, Result,
};
use chrono::Utc;
use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info};
use ulid::Ulid;

/// Events emitted by the process manager to the daemon.
#[derive(Debug)]
pub enum ProcessEvent {
    /// Agent started successfully.
    Started {
        agent_name: String,
        pid: u32,
        generation_id: u64,
    },
    /// Agent exited.
    Exited {
        agent_name: String,
        generation_id: u64,
        exit_code: Option<i32>,
        signal: Option<i32>,
    },
}

/// State for a managed agent process.
struct AgentProcess {
    pid:           u32,
    generation_id: u64,
    restart_count: u32,
    max_restarts:  u32,
    restart:       RestartPolicy,
    manifest:      AgentManifest,
}

/// The process manager — owns all child processes.
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
    event_tx:  mpsc::Sender<ProcessEvent>,
}

impl ProcessManager {
    pub fn new(event_tx: mpsc::Sender<ProcessEvent>) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
        }
    }

    /// Spawn an agent process.
    pub async fn spawn(&self, agent: &Agent) -> Result<u32> {
        let manifest = &agent.manifest;

        // Build command.
        let mut cmd = Command::new(&manifest.entrypoint);
        cmd.args(&manifest.args);
        cmd.envs(&manifest.env);

        // New process group so we can kill the whole group (plan D7).
        unsafe {
            cmd.pre_exec(|| {
                // Create new process group (setsid creates new session + process group).
                libc::setsid();
                // Die when parent dies (Linux-specific).
                #[cfg(target_os = "linux")]
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM, 0, 0, 0);
                Ok(())
            });
        }

        let child = cmd.spawn().map_err(|e| Error::Process(format!("spawn failed: {e}")))?;
        let pid = child.id().ok_or_else(|| Error::Process("no PID".into()))?;

        info!(agent = agent.name, pid, "agent spawned");

        // Watch child in background.
        let name = agent.name.clone();
        let generation_id = agent.generation_id + 1;
        let max_restarts = manifest.max_restarts;
        let restart = manifest.restart.clone();
        let event_tx = self.event_tx.clone();
        let processes = self.processes.clone();
        let manifest_clone = manifest.clone();

        {
            let mut procs = self.processes.lock().map_err(|e| Error::Process(e.to_string()))?;
            procs.insert(name.clone(), AgentProcess {
                pid,
                generation_id,
                restart_count: 0,
                max_restarts,
                restart: restart.clone(),
                manifest: manifest_clone,
            });
        }

        // Notify daemon: started.
        let _ = event_tx.send(ProcessEvent::Started {
            agent_name: name.clone(),
            pid,
            generation_id,
        }).await;

        // Watch in background task.
        tokio::spawn(Self::watch(
            name,
            child,
            generation_id,
            event_tx,
            processes,
        ));

        Ok(pid)
    }

    /// Kill an agent by name (SIGTERM → SIGKILL after 5s).
    pub async fn kill(&self, name: &str) -> Result<()> {
        let pid = {
            let procs = self.processes.lock().map_err(|e| Error::Process(e.to_string()))?;
            procs.get(name).map(|p| p.pid)
        };
        if let Some(pid) = pid {
            // Kill the whole process group.
            unsafe { libc::kill(-(pid as i32), libc::SIGTERM); }
            tokio::time::sleep(Duration::from_secs(5)).await;
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
        }
        Ok(())
    }

    /// Watch a child process and emit events when it exits.
    async fn watch(
        name: String,
        mut child: tokio::process::Child,
        generation_id: u64,
        event_tx: mpsc::Sender<ProcessEvent>,
        _processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
    ) {
        match child.wait().await {
            Ok(status) => {
                use std::os::unix::process::ExitStatusExt;
                let exit_code = status.code();
                let signal    = status.signal();
                info!(
                    agent = name,
                    generation_id,
                    ?exit_code,
                    ?signal,
                    "agent exited"
                );
                let _ = event_tx.send(ProcessEvent::Exited {
                    agent_name: name,
                    generation_id,
                    exit_code,
                    signal,
                }).await;
            }
            Err(e) => {
                error!(agent = name, "wait() error: {e}");
                let _ = event_tx.send(ProcessEvent::Exited {
                    agent_name: name,
                    generation_id,
                    exit_code: None,
                    signal: None,
                }).await;
            }
        }
    }

    /// Compute backoff duration for restart attempt N (exponential + jitter).
    pub fn backoff(attempt: u32) -> Duration {
        use std::time::{SystemTime, UNIX_EPOCH};
        let base_ms: u64 = 1000; // 1 second
        let max_ms: u64 = 60_000; // 60 seconds cap
        let exp = base_ms.saturating_mul(2u64.saturating_pow(attempt.min(10)));
        let capped = exp.min(max_ms);
        // Simple pseudo-jitter using current nanos.
        let jitter = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 % 1000)
            .unwrap_or(0);
        Duration::from_millis(capped + jitter)
    }
}

/// Build an AgentIncident from an exit event.
pub fn make_incident(
    agent: &Agent,
    exit_code: Option<i32>,
    signal: Option<i32>,
    reason: &str,
) -> AgentIncident {
    AgentIncident {
        id:            Ulid::new().to_string(),
        agent_id:      agent.id.clone(),
        agent_name:    agent.name.clone(),
        generation_id: agent.generation_id,
        exit_code,
        signal,
        reason:        reason.into(),
        occurred_at:   Utc::now(),
    }
}
