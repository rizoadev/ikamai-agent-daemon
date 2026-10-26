# Architecture Document

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-26  
**Author:** Rizoa  
**Document ID:** IAD-ARCH-001

---

## 1. System Context

```
┌─────────────────────────────────────────────────────────────────────┐
│                         EXTERNAL WORLD                              │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────────────┐  │
│  │ Operator │  │ CI/CD    │  │ External │  │ Other IAD Nodes    │  │
│  │ (Human)  │  │ Pipeline │  │ Webhooks │  │ (Multi-Node Fleet) │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬───────────┘  │
│       │CLI           │REST        │REST/WS          │gRPC          │
└───────┼──────────────┼────────────┼─────────────────┼───────────────┘
        │              │            │                 │
┌───────┴──────────────┴────────────┴─────────────────┴───────────────┐
│                    IKAMAI AGENT DAEMON (IAD)                         │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   API GATEWAY LAYER                          │    │
│  │  ┌──────┐  ┌──────┐  ┌──────────┐  ┌──────┐  ┌──────────┐  │    │
│  │  │ CLI  │  │ REST │  │ WebSocket│  │ gRPC │  │  Metrics │  │    │
│  │  │:stdin│  │:7700 │  │ :7700/ws │  │:7701 │  │  :7702   │  │    │
│  │  └──┬───┘  └──┬───┘  └────┬─────┘  └──┬───┘  └──┬───────┘  │    │
│  └─────┼─────────┼───────────┼────────────┼─────────┼──────────┘    │
│        └─────────┴───────────┴────────────┴─────────┘               │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐    │
│  │                    SERVICE LAYER                              │    │
│  │                                                               │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐  │    │
│  │  │ Agent    │ │ Task     │ │ Event    │ │ Policy         │  │    │
│  │  │ Registry │ │ Router   │ │ Bus      │ │ Engine         │  │    │
│  │  │          │ │          │ │          │ │                │  │    │
│  │  │•register │ │•submit   │ │•publish  │ │•rate_limit     │  │    │
│  │  │•discover │ │•route    │ │•subscribe│ │•resource_budget│  │    │
│  │  │•health   │ │•retry    │ │•replay   │ │•permissions    │  │    │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └───────┬────────┘  │    │
│  │       │            │            │               │            │    │
│  │  ┌────┴────┐  ┌────┴────┐                                    │    │
│  │  │Scheduler│  │ Health  │                                    │    │
│  │  │         │  │ Monitor │                                    │    │
│  │  │•cron    │  │•heartbeat│                                   │    │
│  │  │•one-shot│  │•circuit  │                                   │    │
│  │  │•DAG deps│  │ breaker  │                                   │    │
│  │  └─────────┘  └──────────┘                                   │    │
│  └──────────────────────────────────────────────────────────────┘    │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐    │
│  │                    INFRASTRUCTURE LAYER                       │    │
│  │                                                               │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐    │    │
│  │  │ Storage  │ │ Process  │ │   IPC    │ │ Observability│    │    │
│  │  │ (SQLite/ │ │ Manager  │ │ Server   │ │ (tracing +   │    │    │
│  │  │  Postgres│ │ (spawn/  │ │ (Unix    │ │  OTel)       │    │    │
│  │  │  )       │ │  kill/   │ │  sockets)│ │              │    │    │
│  │  │          │ │  signal) │ │          │ │              │    │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘    │    │
│  └──────────────────────────────────────────────────────────────┘    │
│                              │                                      │
│  ┌───────────────────────────┴─────────────────────────────────┐    │
│  │                    AGENT LAYER                                │    │
│  │                                                               │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐          │    │
│  │  │ Agent A │ │ Agent B │ │ Agent C │ │ Agent D  │  ...     │    │
│  │  │ Python  │ │ Node.js │ │ Rust    │ │ WASM     │          │    │
│  │  │ PID=123 │ │ PID=456 │ │ PID=789 │ │ Sandbox  │          │    │
│  │  └─────────┘ └─────────┘ └─────────┘ └──────────┘          │    │
│  └──────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Crate Architecture

```
ikamai-agent-daemon/          # Cargo workspace root
├── crates/
│   ├── iad/                  # Binary crate — CLI entry point
│   │   └── src/main.rs       # clap CLI → dispatches to services
│   │
│   ├── iad-core/             # Library crate — domain logic
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs     # TOML config parsing + validation
│   │       ├── error.rs      # Domain error types (thiserror)
│   │       ├── models.rs     # Agent, Task, Event, Schedule structs
│   │       ├── daemon.rs     # Main daemon process (daemonize, PID, signals)
│   │       ├── registry.rs   # Agent registry (CRUD, capability index)
│   │       ├── process.rs    # Process manager (spawn, kill, signal)
│   │       ├── router.rs     # Task router (capability match, load balance)
│   │       ├── eventbus.rs   # Event bus (pub/sub, filter, replay)
│   │       ├── scheduler.rs  # Cron + one-shot scheduler
│   │       ├── health.rs     # Health monitor (heartbeat, circuit breaker)
│   │       └── policy.rs     # Policy engine (rate limits, budgets)
│   │
│   ├── iad-storage/          # Library crate — persistence
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── sqlite.rs     # SQLite backend
│   │       ├── postgres.rs   # PostgreSQL backend (Phase 4)
│   │       ├── migrations/   # SQL migration files
│   │       └── traits.rs     # Storage trait (backend-agnostic)
│   │
│   ├── iad-api/              # Library crate — API servers
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── rest.rs       # Axum REST handlers
│   │       ├── websocket.rs  # WebSocket handler
│   │       ├── grpc.rs       # tonic gRPC server (Phase 4)
│   │       ├── auth.rs       # Authentication middleware
│   │       └── metrics.rs    # Prometheus metrics endpoint
│   │
│   └── iad-sdk/              # Library crate — Agent SDK (Rust)
│       └── src/
│           ├── lib.rs
│           ├── client.rs     # Agent client (connects to daemon)
│           ├── protocol.rs   # Wire protocol (frame encode/decode)
│           └── macros.rs     # Derive macros for agent registration
│
├── sdks/                     # Non-Rust SDKs
│   ├── python/               # Python SDK (Phase 2)
│   │   ├── iad_sdk/
│   │   ├── pyproject.toml
│   │   └── tests/
│   └── node/                 # Node.js SDK (Phase 3)
│       ├── src/
│       ├── package.json
│       └── tests/
│
├── proto/                    # gRPC proto definitions (Phase 4)
│   └── iad/
│       ├── daemon.proto
│       └── agent.proto
│
├── examples/                 # Example agents
│   ├── echo-agent/           # Minimal echo agent (Rust)
│   ├── python-agent/         # Python agent example
│   └── node-agent/           # Node.js agent example
│
├── tests/                    # Integration & E2E tests
│   ├── common/mod.rs         # Test helpers (TestDaemon, fixtures)
│   ├── agent_lifecycle.rs
│   ├── task_routing.rs
│   ├── event_bus.rs
│   └── e2e/
│       └── run_all.py
│
├── docs/                     # Documentation
├── .github/workflows/        # CI/CD
├── Cargo.toml                # Workspace manifest
├── Cargo.lock
├── Dockerfile
├── README.md
├── LICENSE
└── CHANGELOG.md
```

---

## 3. Module Dependency Graph

```
iad (binary)
 ├── iad-core
 ├── iad-storage
 ├── iad-api
 └── iad-sdk

iad-core (domain logic — zero external I/O dependencies)
 └── (no internal crate deps)

iad-storage (persistence)
 └── iad-core (for models, errors)

iad-api (HTTP/gRPC servers)
 ├── iad-core (for models, services)
 └── iad-storage (for data access)

iad-sdk (agent-side)
 └── iad-core (for models, protocol)
```

**Rule:** Dependency arrows go **inward** only. `iad-core` has zero internal dependencies — it defines the domain.

---

## 4. Concurrency Model

### 4.1 Tokio Runtime

```
┌────────────────────────────────────────────┐
│              Tokio Runtime                  │
│                                            │
│  Worker Thread 1 ──► [Task] [Task] [Task]  │
│  Worker Thread 2 ──► [Task] [Task]         │
│  Worker Thread 3 ──► [Task] [Task] [Task]  │
│  Worker Thread 4 ──► [Task]                │
│                                            │
│  Blocking Pool ────► [SQLite write]        │
│                      [Process spawn]       │
│                      [File I/O]            │
└────────────────────────────────────────────┘
```

### 4.2 Key Async Tasks

| Task | Type | Lifetime |
|------|------|----------|
| API server (REST) | Long-running | Daemon lifetime |
| WebSocket server | Long-running | Daemon lifetime |
| gRPC server | Long-running | Daemon lifetime |
| Health monitor loop | Long-running | Daemon lifetime |
| Scheduler loop | Long-running | Daemon lifetime |
| IPC listener (Unix socket) | Long-running | Daemon lifetime |
| Agent health check | Periodic (per agent) | Agent lifetime |
| Task execution monitor | Per-task | Task lifetime |
| Event delivery | Per-event | Fire-and-forget |

### 4.3 Synchronization

| Resource | Lock Type | Rationale |
|----------|-----------|-----------|
| Agent registry | `RwLock<HashMap>` | Many reads, few writes |
| Task queue | `Mutex<VecDeque>` | FIFO ordering matters |
| Event subscribers | `RwLock<Vec>` | Many reads, occasional add/remove |
| SQLite connection | `tokio::sync::Semaphore` | Bounded concurrency |
| Config | `ArcSwap` | Lock-free hot reload |

---

## 5. Storage Architecture

### 5.1 SQLite (Default, Single-Node)

```
$IAD_HOME/data/
├── iad.db              # Main database
├── iad.db-wal          # Write-Ahead Log
└── iad.db-shm          # Shared memory
```

**WAL mode** for concurrent reads + single writer.

### 5.2 Schema

See [SPEC.md](SPEC.md) Section 2 for complete schema definitions.

### 5.3 Migration Strategy

```
crates/iad-storage/src/migrations/
├── 001_initial.sql         # agents, tasks, events, schedules
├── 002_add_labels.sql      # agent labels, task labels
├── 003_add_audit_log.sql   # audit log table
└── ...
```

- Migrations run automatically on daemon start
- Forward-only (no rollback)
- Each migration is idempotent (uses `IF NOT EXISTS`)

---

## 6. Security Architecture

### 6.1 Trust Boundaries

```
┌─────────────────────────────────────────────┐
│ TRUSTED ZONE (same user, same machine)      │
│                                             │
│  CLI ──(Unix socket)──► Daemon              │
│  Agent ──(Unix socket)──► Daemon            │
│                                             │
│  Authentication: uid/gid match (OS-level)   │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ SEMI-TRUSTED ZONE (network, authenticated)  │
│                                             │
│  REST API ──(HTTP + Bearer token)──► Daemon │
│  WebSocket ──(WS + Bearer token)──► Daemon  │
│  gRPC ──(mTLS)──► Daemon                    │
│                                             │
│  Authentication: token validation / cert    │
│  Authorization: RBAC (Phase 4+)             │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ UNTRUSTED ZONE (sandboxed)                  │
│                                             │
│  WASM agents ──(wasmtime sandbox)──► Daemon │
│                                             │
│  Capabilities: defined by host, minimal     │
│  No filesystem, no network, no syscalls     │
└─────────────────────────────────────────────┘
```

### 6.2 Agent Isolation Levels

| Level | Mechanism | Use Case |
|-------|-----------|----------|
| Level 0 | Same process (WASM) | Untrusted plugins |
| Level 1 | Separate process (PID isolation) | Trusted agents (default) |
| Level 2 | cgroups v2 (resource limits) | Resource-sensitive agents |
| Level 3 | Separate user (UID isolation) | Multi-tenant agents |

---

## 7. Observability Architecture

```
┌──────────────────────────────────────────┐
│              IAD Daemon                   │
│                                          │
│  tracing crate ──► Structured JSON logs  │
│                ──► OpenTelemetry spans   │
│                                          │
│  metrics ──► Prometheus /metrics         │
│          ──► OpenTelemetry metrics       │
│                                          │
│  audit log ──► SQLite/Postgres           │
└────┬─────────────┬──────────────┬────────┘
     │             │              │
┌────▼────┐  ┌─────▼─────┐  ┌────▼────────┐
│ stdout/ │  │ Prometheus│  │ OTel        │
│ stderr  │  │ + Grafana │  │ Collector   │
│ + file  │  │           │  │ → Jaeger/   │
│         │  │           │  │   Datadog/  │
│         │  │           │  │   etc.      │
└─────────┘  └───────────┘  └─────────────┘
```

### 7.1 Log Levels

| Level | Use |
|-------|-----|
| `ERROR` | Unrecoverable failures, data loss risk |
| `WARN` | Degraded operation, retry, circuit breaker |
| `INFO` | Agent lifecycle events, task completion, config changes |
| `DEBUG` | Detailed operation flow, IPC messages |
| `TRACE` | Wire protocol bytes, SQL queries, internal state |

---

## 8. Deployment Architecture

### 8.1 Single Node

```
┌────────────────────────────────────┐
│           Single Server            │
│                                    │
│  systemd ──manages──► iad daemon   │
│                        │           │
│                   ┌────┴────┐      │
│                   │ agents  │      │
│                   │ A B C D │      │
│                   └─────────┘      │
│                                    │
│  SQLite (local)                    │
└────────────────────────────────────┘
```

### 8.2 Multi-Node Fleet

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│   Node A     │   │   Node B     │   │   Node C     │
│   (Primary)  │   │   (Worker)   │   │   (Worker)   │
│              │   │              │   │              │
│  iad daemon ◄├──gRPC──►iad daemon◄├──gRPC──►iad daemon│
│   │         │   │   │          │   │   │          │
│  A1 A2 A3   │   │  B1 B2 B3   │   │  C1 C2      │
│              │   │              │   │              │
│  SQLite      │   │  SQLite      │   │  SQLite      │
└──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
                 ┌────────▼────────┐
                 │   PostgreSQL    │
                 │   (shared state)│
                 └─────────────────┘
```

---

## 9. ADR (Architecture Decision Records)

### ADR-001: Rust as Primary Language
- **Decision:** Use Rust for the daemon
- **Rationale:** Memory safety without GC, zero-cost abstractions, fearless concurrency, excellent performance for long-running daemons
- **Alternatives considered:** Go (simpler but less control), C++ (unsafe), Python (too slow for daemon)
- **Date:** 2026-10-26

### ADR-002: SQLite First, PostgreSQL Later
- **Decision:** Start with SQLite, add PostgreSQL for multi-node
- **Rationale:** SQLite requires zero setup, embedded, single-file backup; PostgreSQL adds complexity but enables multi-node
- **Date:** 2026-10-26

### ADR-003: Unix Domain Sockets for Local IPC
- **Decision:** Use UDS for agent ↔ daemon communication on same host
- **Rationale:** Zero-copy, kernel-optimized, <1μs latency, no TCP overhead, natural UID/GID auth
- **Date:** 2026-10-26

### ADR-004: Axum + tonic for API Layer
- **Decision:** Use Axum for REST and tonic for gRPC
- **Rationale:** Both built on Tokio/hyper, composable Tower middleware, type-safe, excellent performance
- **Date:** 2026-10-26

### ADR-005: WASM for Plugin Agents
- **Decision:** Use wasmtime for sandboxed agent plugins
- **Rationale:** Language-agnostic, sandboxed by default, deterministic execution, portable
- **Date:** 2026-10-26

---

*Architecture maintained by Rizoa. Updated with each significant design change.*
