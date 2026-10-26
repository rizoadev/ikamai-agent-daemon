//! Cron + one-shot scheduler (task 2.10).

use crate::{
    models::{Priority, Task, TaskStatus},
    task_queue::TaskQueue,
    Result,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use ulid::Ulid;

/// A scheduled job entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id:           String,
    pub name:         String,
    /// Cron expression (e.g. "0 * * * *") or "once".
    pub schedule:     String,
    pub payload:      serde_json::Value,
    pub capabilities: Vec<String>,
    pub priority:     Priority,
    pub enabled:      bool,
    pub created_at:   chrono::DateTime<Utc>,
    pub last_run:     Option<chrono::DateTime<Utc>>,
    pub next_run:     Option<chrono::DateTime<Utc>>,
}

/// The scheduler — manages cron + one-shot jobs.
pub struct Scheduler {
    jobs:  Mutex<HashMap<String, ScheduledJob>>,
    queue: Arc<TaskQueue>,
}

impl Scheduler {
    pub fn new(queue: Arc<TaskQueue>) -> Arc<Self> {
        Arc::new(Self {
            jobs:  Mutex::new(HashMap::new()),
            queue,
        })
    }

    /// Add a scheduled job.
    pub async fn add(&self, name: &str, schedule: &str, payload: serde_json::Value, capabilities: Vec<String>, priority: Priority) -> ScheduledJob {
        let now = Utc::now();
        let next_run = compute_next_run(schedule, &now);
        let job = ScheduledJob {
            id:           Ulid::new().to_string(),
            name:         name.into(),
            schedule:     schedule.into(),
            payload,
            capabilities,
            priority,
            enabled:      true,
            created_at:   now,
            last_run:     None,
            next_run,
        };
        let mut jobs = self.jobs.lock().await;
        jobs.insert(job.id.clone(), job.clone());
        job
    }

    /// Remove a job by id.
    pub async fn remove(&self, id: &str) -> bool {
        let mut jobs = self.jobs.lock().await;
        jobs.remove(id).is_some()
    }

    /// List all jobs.
    pub async fn list(&self) -> Vec<ScheduledJob> {
        self.jobs.lock().await.values().cloned().collect()
    }

    /// Trigger a job immediately (one-shot manual trigger).
    pub async fn trigger(&self, id: &str) -> Result<String> {
        let jobs = self.jobs.lock().await;
        let job = jobs.get(id)
            .ok_or_else(|| crate::Error::TaskNotFound { id: id.into() })?;
        let task = make_task(job);
        let task_id = task.id.clone();
        self.queue.enqueue(task).await?;
        info!(job_id = id, task_id, "job triggered manually");
        Ok(task_id)
    }

    /// Background tick loop — fires due jobs every second.
    pub async fn run(self: Arc<Self>) {
        loop {
            sleep(Duration::from_secs(1)).await;
            let now = Utc::now();
            let due: Vec<ScheduledJob> = {
                let jobs = self.jobs.lock().await;
                jobs.values()
                    .filter(|j| j.enabled && j.next_run.map(|n| n <= now).unwrap_or(false))
                    .cloned()
                    .collect()
            };

            for job in due {
                let task = make_task(&job);
                let task_id = task.id.clone();
                match self.queue.enqueue(task).await {
                    Ok(_) => {
                        info!(job = job.name, task_id, "scheduled job fired");
                        let mut jobs = self.jobs.lock().await;
                        if let Some(j) = jobs.get_mut(&job.id) {
                            j.last_run = Some(now);
                            if job.schedule == "once" {
                                j.enabled  = false;
                                j.next_run = None;
                            } else {
                                j.next_run = compute_next_run(&job.schedule, &now);
                            }
                        }
                    }
                    Err(e) => warn!(job = job.name, "failed to enqueue job: {e}"),
                }
            }
        }
    }
}

fn make_task(job: &ScheduledJob) -> Task {
    let now = Utc::now();
    Task {
        id:             Ulid::new().to_string(),
        title:          Some(format!("scheduled:{}", job.name)),
        payload:        job.payload.clone(),
        capabilities:   job.capabilities.clone(),
        priority:       job.priority,
        status:         TaskStatus::Pending,
        assigned_agent: None,
        result:         None,
        error:          None,
        retry_count:    0,
        max_retries:    3,
        created_at:     now,
        updated_at:     now,
    }
}

/// Compute next run time for a cron expression.
/// Returns None for "once" (run immediately → next_run = now).
fn compute_next_run(schedule: &str, from: &chrono::DateTime<Utc>) -> Option<chrono::DateTime<Utc>> {
    if schedule == "once" {
        return Some(*from);
    }
    // Parse cron expression.
    match schedule.parse::<cron::Schedule>() {
        Ok(s) => s.after(from).next(),
        Err(e) => {
            warn!("invalid cron expression '{schedule}': {e}");
            None
        }
    }
}
