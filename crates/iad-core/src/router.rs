//! Capability-based task router (task 2.5).

use crate::{
    models::{Agent, AgentStatus, Task, TaskStatus},
    registry::AgentRegistry,
    task_queue::TaskQueue,
    Error, Result,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Routes tasks from queue to capable agents.
pub struct TaskRouter {
    queue:    Arc<TaskQueue>,
    registry: Arc<AgentRegistry>,
}

impl TaskRouter {
    pub fn new(queue: Arc<TaskQueue>, registry: Arc<AgentRegistry>) -> Self {
        Self { queue, registry }
    }

    /// Background loop — polls queue, assigns tasks to agents.
    pub async fn run(self: Arc<Self>) {
        loop {
            self.queue.wait_for_task().await;
            self.try_dispatch().await;
        }
    }

    /// Try to dispatch pending tasks to available agents.
    async fn try_dispatch(&self) {
        let agents = self.registry.list().await;
        let running: Vec<Agent> = agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Running)
            .collect();

        if running.is_empty() {
            warn!("no running agents, tasks remain queued");
            return;
        }

        for agent in &running {
            if let Some(task) = self.queue.dequeue_for(agent).await {
                let lease = self.queue.acquire_lease(&task.id, &agent.name).await;
                info!(
                    task_id = task.id,
                    agent   = agent.name,
                    lease   = lease.lease_token,
                    "task assigned"
                );
                // Mark task as assigned in storage (caller handles persistence).
                // In Phase 2 IPC: send AssignTask frame to agent.
                debug!(task_id = task.id, agent = agent.name, "AssignTask dispatched");
            }
        }
    }

    /// Handle task completion.
    pub async fn complete(&self, task_id: &str) {
        self.queue.release_lease(task_id).await;
        info!(task_id, "task completed");
    }

    /// Handle task failure with retry logic.
    pub async fn fail(
        &self,
        mut task: Task,
        error: &str,
        error_class: &crate::ipc::ErrorClass,
    ) {
        self.queue.release_lease(&task.id).await;
        if crate::task_queue::should_retry(&task, error_class) {
            task.retry_count += 1;
            task.status = TaskStatus::Pending;
            warn!(task_id = task.id, retry = task.retry_count, "retrying task");
            let _ = self.queue.enqueue(task).await;
        } else {
            warn!(task_id = task.id, error, "task dead-lettered");
            task.status = TaskStatus::DeadLetter;
            self.queue.dead_letter(task).await;
        }
    }
}
