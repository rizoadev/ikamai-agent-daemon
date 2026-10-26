#[cfg(test)]
mod tests {
    use crate::task_queue::{TaskQueue, should_retry, MAX_QUEUE_SIZE};
    use crate::ipc::ErrorClass;
    use crate::models::{Priority, Task, TaskStatus};
    use chrono::Utc;
    use ulid::Ulid;

    fn make_task(caps: &[&str], priority: Priority) -> Task {
        let now = Utc::now();
        Task {
            id: Ulid::new().to_string(),
            title: None,
            payload: serde_json::json!({"test": true}),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            priority,
            status: TaskStatus::Pending,
            assigned_agent: None,
            result: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn enqueue_and_dequeue_basic() {
        use crate::models::{Agent, AgentManifest, AgentStatus, RestartPolicy};
        let q = TaskQueue::new();
        let task = make_task(&["echo"], Priority::Medium);
        q.enqueue(task.clone()).await.unwrap();
        assert_eq!(q.len().await, 1);

        // Create a fake agent with matching capability
        let manifest = AgentManifest {
            name: "a".into(), version: "1.0.0".into(), runtime: "bash".into(),
            entrypoint: "/bin/echo".into(), args: vec![], capabilities: vec!["echo".into()],
            env: Default::default(), restart: RestartPolicy::OnFailure, max_restarts: 3,
        };
        let agent = Agent {
            id: Ulid::new().to_string(), name: "a".into(), version: "1.0.0".into(),
            status: AgentStatus::Running, pid: None, manifest,
            generation_id: 0, failure_count: 0, started_at: None, stopped_at: None,
            created_at: Utc::now(), updated_at: Utc::now(),
        };
        let got = q.dequeue_for(&agent).await;
        assert!(got.is_some());
        assert_eq!(got.unwrap().id, task.id);
        assert_eq!(q.len().await, 0);
    }

    #[tokio::test]
    async fn priority_ordering() {
        use crate::models::{Agent, AgentManifest, AgentStatus, RestartPolicy};
        let q = TaskQueue::new();
        q.enqueue(make_task(&["x"], Priority::Low)).await.unwrap();
        q.enqueue(make_task(&["x"], Priority::Urgent)).await.unwrap();
        q.enqueue(make_task(&["x"], Priority::Medium)).await.unwrap();

        let manifest = AgentManifest {
            name: "b".into(), version: "1.0.0".into(), runtime: "bash".into(),
            entrypoint: "/bin/echo".into(), args: vec![], capabilities: vec!["x".into()],
            env: Default::default(), restart: RestartPolicy::OnFailure, max_restarts: 3,
        };
        let agent = Agent {
            id: Ulid::new().to_string(), name: "b".into(), version: "1.0.0".into(),
            status: AgentStatus::Running, pid: None, manifest,
            generation_id: 0, failure_count: 0, started_at: None, stopped_at: None,
            created_at: Utc::now(), updated_at: Utc::now(),
        };
        // Urgent (1) should come first
        let first = q.dequeue_for(&agent).await.unwrap();
        assert_eq!(first.priority, Priority::Urgent);
    }

    #[tokio::test]
    async fn backpressure_at_max() {
        let q = TaskQueue::default();
        for i in 0..MAX_QUEUE_SIZE {
            q.enqueue(make_task(&["y"], Priority::Low)).await.unwrap();
        }
        let result = q.enqueue(make_task(&["y"], Priority::Low)).await;
        assert!(result.is_err(), "should reject when full");
    }

    #[tokio::test]
    async fn lease_acquire_renew_release() {
        let q = TaskQueue::new();
        let task = make_task(&["z"], Priority::Medium);
        let tid = task.id.clone();
        q.enqueue(task).await.unwrap();

        let lease = q.acquire_lease(&tid, "agent-a").await;
        assert_eq!(lease.task_id, tid);
        assert!(!lease.lease_token.is_empty());

        // Renew with correct token
        assert!(q.renew_lease(&tid, &lease.lease_token).await);
        // Renew with wrong token
        assert!(!q.renew_lease(&tid, "wrong-token").await);

        q.release_lease(&tid).await;
        // After release, reclaim should find nothing
        let expired = q.reclaim_expired().await;
        assert!(!expired.contains(&tid));
    }

    #[tokio::test]
    async fn dead_letter_and_retry() {
        let q = TaskQueue::new();
        let task = make_task(&["dl"], Priority::Low);
        let tid = task.id.clone();
        q.dead_letter(task).await;

        let dl = q.list_dead_letter().await;
        assert_eq!(dl.len(), 1);

        q.retry_dead_letter(&tid).await.unwrap();
        assert_eq!(q.list_dead_letter().await.len(), 0);
        assert_eq!(q.len().await, 1);
    }

    #[test]
    fn should_retry_transient() {
        let task = make_task(&[], Priority::Low);
        assert!(should_retry(&task, &ErrorClass::Transient));
    }

    #[test]
    fn should_not_retry_fatal() {
        let task = make_task(&[], Priority::Low);
        assert!(!should_retry(&task, &ErrorClass::Fatal));
    }

    #[test]
    fn should_not_retry_exhausted() {
        let mut task = make_task(&[], Priority::Low);
        task.retry_count = 3; // == max_retries
        assert!(!should_retry(&task, &ErrorClass::Transient));
    }
}
