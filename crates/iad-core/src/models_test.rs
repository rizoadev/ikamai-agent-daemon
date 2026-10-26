#[cfg(test)]
mod tests {
    use crate::{AgentManifest, AgentStatus, Priority, RestartPolicy, TaskStatus};

    #[test]
    fn agent_status_display() {
        assert_eq!(AgentStatus::Running.to_string(), "running");
        assert_eq!(AgentStatus::Crashed.to_string(), "crashed");
        assert_eq!(AgentStatus::Registered.to_string(), "registered");
    }

    #[test]
    fn task_status_display() {
        assert_eq!(TaskStatus::DeadLetter.to_string(), "dead_letter");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn agent_manifest_defaults() {
        let yaml = r#"
name: test
version: "1.0.0"
runtime: bash
entrypoint: /bin/echo
"#;
        let m: AgentManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.name, "test");
        assert!(m.capabilities.is_empty());
        assert_eq!(m.max_restarts, 5);
        assert_eq!(m.restart, RestartPolicy::OnFailure);
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Urgent < Priority::High);
        assert!(Priority::High < Priority::Medium);
    }
}
