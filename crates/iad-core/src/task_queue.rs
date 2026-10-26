//! Bounded task queue with backpressure (plan task 2.4).
//! Capability-based routing (task 2.5).
//! Lease system with TTL (task 2.R2, plan D8).
//! Retry + error classification + dead-letter (tasks 2.7, 2.R1).

use crate::{
    ipc::ErrorClass,
    models::{Agent, Priority, Task, TaskStatus},
    Error, Result,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use ulid::Ulid;

/// Max tasks in queue before backpressure kicks in.
pub const MAX_QUEUE_SIZE: usize = 10_000;

/// Task lease TTL in seconds.
pub const LEASE_TTL_SECS: i64 = 60;

/// Active lease record.
#[derive(Debug, Clone)]
pub struct Lease {
    pub task_id:     String,
    pub agent_name:  String,
    pub lease_token: String,
    pub expires_at:  DateTime<Utc>,
}

/// The task queue — priority-ordered, bounded, with leases.
pub struct TaskQueue {
    /// Priority buckets: Urgent(1) → Low(4).
    pending:    Mutex<BTreeMap<u8, VecDeque<Task>>>,
    /// Active leases: task_id → Lease.
    leases:     Mutex<HashMap<String, Lease>>,
    /// Dead-letter queue.
    dead_letter: Mutex<Vec<Task>>,
    /// Wakes up the router when new tasks arrive.
    notify:     Notify,
    /// Total pending count (approximate).
    count:      Mutex<usize>,
}

impl TaskQueue {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            pending:     Mutex::new(BTreeMap::new()),
            leases:      Mutex::new(HashMap::new()),
            dead_letter: Mutex::new(Vec::new()),
            notify:      Notify::new(),
            count:       Mutex::new(0),
        })
    }

    /// Enqueue a task. Returns Err if queue is full (backpressure).
    pub async fn enqueue(&self, task: Task) -> Result<()> {
        let mut count = self.count.lock().await;
        if *count >= MAX_QUEUE_SIZE {
            return Err(Error::Process(format!(
                "task queue full ({MAX_QUEUE_SIZE} tasks), backpressure"
            )));
        }
        let prio = task.priority as u8;
        let mut pending = self.pending.lock().await;
        pending.entry(prio).or_default().push_back(task);
        *count += 1;
        drop(count);
        drop(pending);
        self.notify.notify_one();
        Ok(())
    }

    /// Dequeue highest-priority task for an agent with matching capabilities.
    pub async fn dequeue_for(&self, agent: &Agent) -> Option<Task> {
        let mut pending = self.pending.lock().await;
        // Iterate priority buckets lowest number first (Urgent=1 first).
        for (_prio, bucket) in pending.iter_mut() {
            let pos = bucket.iter().position(|t| {
                t.capabilities.iter().all(|c| agent.manifest.capabilities.contains(c))
            });
            if let Some(pos) = pos {
                let task = bucket.remove(pos).unwrap();
                drop(pending);
                let mut count = self.count.lock().await;
                *count = count.saturating_sub(1);
                return Some(task);
            }
        }
        None
    }

    /// Wait until a task is available.
    pub async fn wait_for_task(&self) {
        self.notify.notified().await;
    }

    /// Acquire a lease for task assignment.
    pub async fn acquire_lease(&self, task_id: &str, agent_name: &str) -> Lease {
        let lease = Lease {
            task_id:     task_id.into(),
            agent_name:  agent_name.into(),
            lease_token: Ulid::new().to_string(),
            expires_at:  Utc::now() + chrono::Duration::seconds(LEASE_TTL_SECS),
        };
        let mut leases = self.leases.lock().await;
        leases.insert(task_id.into(), lease.clone());
        lease
    }

    /// Renew a lease heartbeat. Returns false if token mismatch.
    pub async fn renew_lease(&self, task_id: &str, token: &str) -> bool {
        let mut leases = self.leases.lock().await;
        if let Some(lease) = leases.get_mut(task_id) {
            if lease.lease_token == token {
                lease.expires_at = Utc::now() + chrono::Duration::seconds(LEASE_TTL_SECS);
                return true;
            }
        }
        false
    }

    /// Release a lease after task completion.
    pub async fn release_lease(&self, task_id: &str) {
        let mut leases = self.leases.lock().await;
        leases.remove(task_id);
    }

    /// Reclaim expired leases — re-enqueue tasks.
    pub async fn reclaim_expired(&self) -> Vec<String> {
        let now = Utc::now();
        let mut leases = self.leases.lock().await;
        let expired: Vec<String> = leases
            .iter()
            .filter(|(_, l)| l.expires_at < now)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            leases.remove(id);
        }
        expired
    }

    /// Move task to dead-letter queue after max retries.
    pub async fn dead_letter(&self, task: Task) {
        let mut dl = self.dead_letter.lock().await;
        dl.push(task);
    }

    /// List dead-letter tasks.
    pub async fn list_dead_letter(&self) -> Vec<Task> {
        self.dead_letter.lock().await.clone()
    }

    /// Re-enqueue a dead-letter task (retry from DLQ).
    pub async fn retry_dead_letter(&self, task_id: &str) -> Result<()> {
        let mut dl = self.dead_letter.lock().await;
        let pos = dl.iter().position(|t| t.id == task_id)
            .ok_or_else(|| Error::TaskNotFound { id: task_id.into() })?;
        let mut task = dl.remove(pos);
        task.status = TaskStatus::Pending;
        task.retry_count = 0;
        drop(dl);
        self.enqueue(task).await
    }

    /// Pending count.
    pub async fn len(&self) -> usize {
        *self.count.lock().await
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self {
            pending:     Mutex::new(BTreeMap::new()),
            leases:      Mutex::new(HashMap::new()),
            dead_letter: Mutex::new(Vec::new()),
            notify:      Notify::new(),
            count:       Mutex::new(0),
        }
    }
}

/// Decide retry policy for a task given error class.
pub fn should_retry(task: &Task, error_class: &ErrorClass) -> bool {
    if !error_class.is_retryable() {
        return false;
    }
    task.retry_count < task.max_retries
}
