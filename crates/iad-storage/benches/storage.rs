//! Benchmark: SQLite storage operations.
use criterion::{criterion_group, criterion_main, Criterion};
use iad_core::models::*;
use iad_storage::Storage;
use chrono::Utc;
use ulid::Ulid;

fn make_agent(name: &str) -> Agent {
    let now = Utc::now();
    Agent {
        id: Ulid::new().to_string(), name: name.into(), version: "1.0.0".into(),
        status: AgentStatus::Registered, pid: None,
        manifest: AgentManifest {
            name: name.into(), version: "1.0.0".into(), runtime: "bash".into(),
            entrypoint: "/bin/echo".into(), args: vec![], capabilities: vec!["bench".into()],
            env: Default::default(), restart: RestartPolicy::OnFailure, max_restarts: 3,
        },
        generation_id: 0, failure_count: 0,
        started_at: None, stopped_at: None, created_at: now, updated_at: now,
    }
}

fn bench_upsert(c: &mut Criterion) {
    let storage = Storage::open(std::path::Path::new(":memory:")).unwrap();
    c.bench_function("storage_upsert_agent_100", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let mut a = make_agent(&format!("agent-{i}"));
            storage.upsert_agent(&a).unwrap();
            i += 1;
        })
    });
}

fn bench_list(c: &mut Criterion) {
    let storage = Storage::open(std::path::Path::new(":memory:")).unwrap();
    for i in 0..100 {
        storage.upsert_agent(&make_agent(&format!("a{i}"))).unwrap();
    }
    c.bench_function("storage_list_agents_100", |b| {
        b.iter(|| storage.list_agents().unwrap())
    });
}

criterion_group!(benches, bench_upsert, bench_list);
criterion_main!(benches);
