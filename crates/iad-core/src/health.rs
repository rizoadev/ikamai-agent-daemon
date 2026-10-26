//! Health monitor — periodically checks agents and triggers restarts.
//!
//! Implements tasks 1.8, 1.9 (exponential backoff + jitter + staggered restart).

use crate::{
    models::{AgentStatus, RestartPolicy},
    process::ProcessManager,
    registry::AgentRegistry,
    DaemonConfig,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn, error};

/// Health monitor — runs as a background Tokio task.
pub struct HealthMonitor {
    registry:        Arc<AgentRegistry>,
    process_manager: Arc<ProcessManager>,
    cfg:             DaemonConfig,
}

impl HealthMonitor {
    pub fn new(
        registry: Arc<AgentRegistry>,
        process_manager: Arc<ProcessManager>,
        cfg: DaemonConfig,
    ) -> Self {
        Self { registry, process_manager, cfg }
    }

    /// Start the monitor loop.
    pub async fn run(self: Arc<Self>) {
        let interval_secs = self.cfg.health.check_interval_secs;
        let mut ticker = time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            self.check_all().await;
        }
    }

    async fn check_all(&self) {
        let agents = self.registry.list().await;
        for agent in agents {
            match agent.status {
                AgentStatus::Running => {
                    // Verify PID is actually alive.
                    if let Some(pid) = agent.pid {
                        let alive = unsafe { libc::kill(pid as i32, 0) == 0 };
                        if !alive {
                            warn!(agent = agent.name, pid, "agent PID gone but status=running, marking crashed");
                            // Daemon's event loop will handle restart via ProcessEvent::Exited.
                        }
                    }
                }
                AgentStatus::Failed | AgentStatus::Crashed => {
                    let policy = &agent.manifest.restart;
                    let max = agent.manifest.max_restarts;
                    let failures = agent.failure_count;

                    if *policy == RestartPolicy::Never {
                        continue;
                    }
                    if max > 0 && failures >= max {
                        error!(
                            agent = agent.name,
                            failures,
                            max,
                            "agent exceeded max_restarts, not restarting"
                        );
                        continue;
                    }

                    let backoff = ProcessManager::backoff(failures);
                    info!(
                        agent = agent.name,
                        attempt = failures + 1,
                        backoff_ms = backoff.as_millis(),
                        "scheduling restart"
                    );

                    // Staggered: sleep backoff then spawn.
                    let registry   = self.registry.clone();
                    let pm         = self.process_manager.clone();
                    let agent_name = agent.name.clone();
                    tokio::spawn(async move {
                        time::sleep(backoff).await;
                        match registry.get(&agent_name).await {
                            Ok(a) => {
                                // Update status to Starting.
                                let _ = registry.update(&agent_name, |ag| {
                                    ag.status        = AgentStatus::Starting;
                                    ag.failure_count += 1;
                                    ag.generation_id += 1;
                                }).await;
                                if let Err(e) = pm.spawn(&a).await {
                                    error!(agent = agent_name, "restart spawn failed: {e}");
                                }
                            }
                            Err(e) => {
                                warn!(agent = agent_name, "agent gone during restart: {e}");
                            }
                        }
                    });
                }
                _ => {}
            }
        }
    }
}
