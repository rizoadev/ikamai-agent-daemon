#[cfg(test)]
mod tests {
    use crate::scheduler::Scheduler;
    use crate::task_queue::TaskQueue;
    use crate::models::Priority;

    #[tokio::test]
    async fn add_list_remove_job() {
        let q = TaskQueue::new();
        let s = Scheduler::new(q);
        // cron crate uses 6-field format: sec min hour day month weekday
        let job = s.add("daily-job", "0 0 9 * * *", serde_json::json!({}), vec!["echo".into()], Priority::Low).await;
        assert_eq!(job.name, "daily-job");
        assert!(job.enabled);
        assert!(job.next_run.is_some() || !job.enabled || job.schedule == "once");

        let jobs = s.list().await;
        assert_eq!(jobs.len(), 1);

        let removed = s.remove(&job.id).await;
        assert!(removed);
        assert!(s.list().await.is_empty());
    }

    #[tokio::test]
    async fn one_shot_job_fires_immediately() {
        let q = TaskQueue::new();
        let s = Scheduler::new(q.clone());
        let job = s.add("once-job", "once", serde_json::json!({"x":1}), vec![], Priority::Urgent).await;
        // Trigger manually
        let task_id = s.trigger(&job.id).await.unwrap();
        assert!(!task_id.is_empty());
        assert_eq!(q.len().await, 1);
    }

    #[tokio::test]
    async fn trigger_nonexistent_fails() {
        let q = TaskQueue::new();
        let s = Scheduler::new(q);
        let r = s.trigger("nonexistent-id").await;
        assert!(r.is_err());
    }
}
