#[cfg(test)]
mod tests {
    use crate::Storage;
    use iad_core::models::*;
    use iad_core::AgentIncident;
    use chrono::Utc;
    use ulid::Ulid;

    fn make_agent(name: &str) -> Agent {
        let manifest = AgentManifest {
            name:         name.into(),
            version:      "1.0.0".into(),
            runtime:      "bash".into(),
            entrypoint:   "/bin/echo".into(),
            args:         vec!["hello".into()],
            capabilities: vec!["echo".into()],
            env:          Default::default(),
            restart:      RestartPolicy::OnFailure,
            max_restarts: 3,
        };
        let now = Utc::now();
        Agent {
            id:            Ulid::new().to_string(),
            name:          name.into(),
            version:       "1.0.0".into(),
            status:        AgentStatus::Registered,
            pid:           None,
            manifest,
            generation_id: 0,
            failure_count: 0,
            started_at:    None,
            stopped_at:    None,
            created_at:    now,
            updated_at:    now,
        }
    }

    fn in_mem_storage() -> Storage {
        Storage::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn upsert_and_get_agent() {
        let s = in_mem_storage();
        let a = make_agent("alpha");
        s.upsert_agent(&a).unwrap();
        let got = s.get_agent_by_name("alpha").unwrap().unwrap();
        assert_eq!(got.name, "alpha");
        assert_eq!(got.version, "1.0.0");
    }

    #[test]
    fn list_agents_empty() {
        let s = in_mem_storage();
        assert!(s.list_agents().unwrap().is_empty());
    }

    #[test]
    fn list_agents_multiple() {
        let s = in_mem_storage();
        s.upsert_agent(&make_agent("a")).unwrap();
        s.upsert_agent(&make_agent("b")).unwrap();
        s.upsert_agent(&make_agent("c")).unwrap();
        let agents = s.list_agents().unwrap();
        assert_eq!(agents.len(), 3);
    }

    #[test]
    fn upsert_updates_status() {
        let s = in_mem_storage();
        let mut a = make_agent("beta");
        s.upsert_agent(&a).unwrap();
        a.status        = AgentStatus::Running;
        a.pid           = Some(1234);
        a.generation_id = 1;
        s.upsert_agent(&a).unwrap();
        let got = s.get_agent_by_name("beta").unwrap().unwrap();
        assert_eq!(got.status, AgentStatus::Running);
        assert_eq!(got.pid, Some(1234));
        assert_eq!(got.generation_id, 1);
    }

    #[test]
    fn delete_agent() {
        let s = in_mem_storage();
        let a = make_agent("gamma");
        s.upsert_agent(&a).unwrap();
        let deleted = s.delete_agent_by_name("gamma").unwrap();
        assert!(deleted);
        assert!(s.get_agent_by_name("gamma").unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_agent() {
        let s = in_mem_storage();
        let deleted = s.delete_agent_by_name("nope").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn insert_and_list_incidents() {
        let s = in_mem_storage();
        let a = make_agent("delta");
        s.upsert_agent(&a).unwrap();
        let inc = AgentIncident {
            id:            Ulid::new().to_string(),
            agent_id:      a.id.clone(),
            agent_name:    "delta".into(),
            generation_id: 1,
            exit_code:     Some(1),
            signal:        None,
            reason:        "OOM".into(),
            occurred_at:   Utc::now(),
        };
        s.insert_incident(&inc).unwrap();
        let incidents = s.list_incidents_for_agent("delta").unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].reason, "OOM");
    }

    #[test]
    fn get_nonexistent_agent() {
        let s = in_mem_storage();
        assert!(s.get_agent_by_name("zzz").unwrap().is_none());
        assert!(s.get_agent("zzz").unwrap().is_none());
    }
}
