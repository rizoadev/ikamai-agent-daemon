//! Benchmark: TaskQueue enqueue/dequeue throughput.
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use iad_core::models::{Priority, Task, TaskStatus};
use iad_core::task_queue::TaskQueue;
use chrono::Utc;
use ulid::Ulid;

fn make_task(prio: Priority) -> Task {
    let now = Utc::now();
    Task {
        id: Ulid::new().to_string(), title: None,
        payload: serde_json::json!({}),
        capabilities: vec!["bench".into()],
        priority: prio, status: TaskStatus::Pending,
        assigned_agent: None, result: None, error: None,
        retry_count: 0, max_retries: 3, created_at: now, updated_at: now,
    }
}

fn bench_enqueue(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("task_queue_enqueue_1000", |b| {
        b.iter(|| {
            rt.block_on(async {
                let q = TaskQueue::default();
                for _ in 0..1000 {
                    q.enqueue(make_task(Priority::Medium)).await.unwrap();
                }
                q.len().await
            })
        })
    });
}

fn bench_lease(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("task_queue_lease_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let q = TaskQueue::default();
                for _ in 0..100 {
                    let t = make_task(Priority::Urgent);
                    let id = t.id.clone();
                    q.enqueue(t).await.unwrap();
                    let lease = q.acquire_lease(&id, "bench-agent").await;
                    q.release_lease(&id).await;
                }
            })
        })
    });
}

criterion_group!(benches, bench_enqueue, bench_lease);
criterion_main!(benches);
