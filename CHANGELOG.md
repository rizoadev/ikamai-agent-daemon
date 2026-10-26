# Changelog

All notable changes to IAD are documented here.
Format: [Semantic Versioning](https://semver.org/)

---

## [Unreleased] — v1.0.0

### Added
- **Phase 0**: GitHub repo, Cargo workspace, CI/CD, pre-commit hooks
- **Phase 1**: Core daemon
  - PID file with flock (TOCTOU-safe)
  - TOML configuration loading
  - SQLite storage (WAL mode, migrations)
  - Agent manifest parser (YAML)
  - Agent registry with generation_id
  - Process manager (setsid, PR_SET_PDEATHSIG, process groups)
  - Agent lifecycle state machine (registered → running → crashed → restart)
  - Health monitor with exponential backoff + jitter
  - Crash recovery on daemon restart
  - Flight recorder (agent_incidents table)
  - Structured JSON logging (tracing)
  - CLI: init, start, stop, status, reload, agent CRUD
- **Phase 2**: Task routing & events
  - Length-prefixed JSON IPC protocol
  - DaemonFrame / AgentFrame / ErrorClass
  - Bounded task queue (10k, backpressure)
  - Priority buckets (Urgent → Low)
  - Lease system (60s TTL, heartbeat renew, reclaim)
  - Dead-letter queue + retry
  - Capability-based task router
  - Event bus (Tokio broadcast, per-topic)
  - Scheduler (cron + one-shot)
  - Python SDK (IadAgent, CancellationToken)
  - Node.js SDK (IadAgent, FrameReader)
- **Phase 3**: API & monitoring
  - Axum REST API (16 endpoints)
  - Bearer token authentication
  - WebSocket event stream (/api/v1/ws, topic filter)
  - Prometheus metrics (/metrics, 10 counters/gauges)
  - Alert webhooks (agent crash, task fail)
  - Hot-reload configuration (SIGHUP)
  - OpenAPI 3.1 specification
  - k6 load test (100 VUs, p95 < 200ms)
- **Phase 4**: Scale & plugins
  - gRPC server (tonic) — AgentService, TaskService, DaemonService
  - Protobuf definitions (iad.proto)
  - WASM plugin runtime (wasmtime, sandboxed, fuel+memory limits)
  - Node discovery + multi-node registry
  - Cross-node task routing + event propagation
  - PostgreSQL storage backend (sqlx)
  - RBAC (Admin/Operator/Reader roles)
  - Secret management (env injection)
  - cgroups v2 resource limits
  - mTLS configuration
- **Phase 5**: Production
  - Docker image + Compose
  - systemd unit file
  - Quick-start guide
  - Agent development guide
  - Deployment guide
  - Example agents (echo, timer, LLM)
  - Performance benchmarks (criterion)
  - Fuzz tests (cargo-fuzz, 3 targets)
  - Security documentation

### Tests
- 38 Rust unit/integration tests
- 4 Node.js protocol tests
- Integration test: full agent lifecycle
- Fuzz targets: IPC frame, manifest, config

---

## [0.1.0-alpha] — 2026-10-26

Initial scaffold release.
