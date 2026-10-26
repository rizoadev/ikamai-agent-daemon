#[cfg(test)]
mod tests {
    use crate::{AgentManifest, AgentStatus, AgentRegistry, RestartPolicy};

    fn manifest(name: &str) -> AgentManifest {
        AgentManifest {
            name:         name.into(),
            version:      "1.0.0".into(),
            runtime:      "bash".into(),
            entrypoint:   "/bin/echo".into(),
            args:         vec![],
            capabilities: vec!["echo".into(), "text".into()],
            env:          Default::default(),
            restart:      RestartPolicy::OnFailure,
            max_restarts: 3,
        }
    }

    #[tokio::test]
    async fn register_and_get() {
        let r = AgentRegistry::new();
        let a = r.register(manifest("agent-a")).await.unwrap();
        assert_eq!(a.name, "agent-a");
        assert_eq!(a.status, AgentStatus::Registered);
        assert_eq!(a.generation_id, 0);
    }

    #[tokio::test]
    async fn register_duplicate_fails() {
        let r = AgentRegistry::new();
        r.register(manifest("dup")).await.unwrap();
        let result = r.register(manifest("dup")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn deregister() {
        let r = AgentRegistry::new();
        r.register(manifest("to-remove")).await.unwrap();
        let removed = r.deregister("to-remove").await.unwrap();
        assert_eq!(removed.name, "to-remove");
        assert!(r.list().await.is_empty());
    }

    #[tokio::test]
    async fn deregister_nonexistent_fails() {
        let r = AgentRegistry::new();
        assert!(r.deregister("ghost").await.is_err());
    }

    #[tokio::test]
    async fn update_agent() {
        let r = AgentRegistry::new();
        r.register(manifest("upd")).await.unwrap();
        let updated = r.update("upd", |a| {
            a.status        = AgentStatus::Running;
            a.pid           = Some(9999);
            a.generation_id = 2;
        }).await.unwrap();
        assert_eq!(updated.status, AgentStatus::Running);
        assert_eq!(updated.pid, Some(9999));
        assert_eq!(updated.generation_id, 2);
    }

    #[tokio::test]
    async fn find_capable() {
        let r = AgentRegistry::new();
        r.register(manifest("cap-a")).await.unwrap();
        r.update("cap-a", |a| a.status = AgentStatus::Running).await.unwrap();
        let caps = r.find_capable(&["echo".into()]).await;
        assert_eq!(caps.len(), 1);
        let caps2 = r.find_capable(&["nonexistent".into()]).await;
        assert!(caps2.is_empty());
    }

    #[tokio::test]
    async fn list_agents() {
        let r = AgentRegistry::new();
        r.register(manifest("x")).await.unwrap();
        r.register(manifest("y")).await.unwrap();
        assert_eq!(r.list().await.len(), 2);
    }
}
