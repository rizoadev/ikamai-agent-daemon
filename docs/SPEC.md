# Technical Specification

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-27  
**Author:** Rizoa  
**Status:** DRAFT  
**See also:** [RESILIENCE.md](RESILIENCE.md) — Comprehensive fault handling specification

---

## 1. System Architecture

### 1.1 Process Model

```
                    ┌─────────────────┐
                    │   iad (daemon)   │
                    │   PID 1 (main)   │
                    │                  │
                    │  ┌────────────┐  │
                    │  │ Event Loop │  │
                    │  │  (Tokio)   │  │
                    │  └─────┬──────┘  │
                    │        │         │
          ┌─────────┼────────┼─────────┼─────────┐
          │         │        │         │         │
     ┌────┴───┐ ┌───┴────┐ ┌┴───────┐ ┌┴──────┐ ┌┴──────┐
     │Registry│ │Router  │ │EventBus│ │Sched. │ │Health │
     │Service │ │Service │ │Service │ │Service│ │Monitor│
     └────────┘ └────────┘ └────────┘ └───────┘ └───────┘
          │         │         │          │         │
     ┌────┴─────────┴─────────┴──────────┴─────────┴────┐
     │              Storage Layer (SQLite)                │
     └───────────────────────────────────────────────────┘
          │
     ┌────┴──────────────────────────────────┐
     │         Agent Processes               │
     │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐    │
     │  │ A1  │ │ A2  │ │ A3  │ │ A4  │    │
     │  │pid=X│ │pid=Y│ │pid=Z│ │pid=W│    │
     │  └─────┘ └─────┘ └─────┘ └─────┘    │
     └───────────────────────────────────────┘
```

### 1.2 Directory Layout

```
$IAD_HOME/                    # Default: ~/.iad/
├── iad.toml                  # Daemon configuration
├── agents/                   # Agent manifests
│   ├── content-writer.yaml
│   ├── data-scraper.yaml
│   └── trading-bot.yaml
├── data/
│   ├── iad.db                # SQLite database
│   └── iad.db-wal            # WAL journal
├── logs/
│   ├── daemon.log            # Daemon structured logs
│   └── agents/
│       ├── content-writer.log
│       └── data-scraper.log
├── run/
│   ├── iad.pid               # Daemon PID file
│   └── iad.sock              # Unix domain socket (API)
└── secrets/
    └── .vault                # Encrypted secrets
```

### 1.3 Daemon Configuration

```toml
# $IAD_HOME/iad.toml

[daemon]
pid_file = "run/iad.pid"
socket = "run/iad.sock"
log_level = "info"                # trace, debug, info, warn, error
log_format = "json"               # json, pretty
workers = 4                       # Tokio worker threads

[storage]
backend = "sqlite"                # sqlite | postgres
path = "data/iad.db"              # SQLite path
# url = "postgres://..."          # PostgreSQL URL (if backend=postgres)

[api]
rest_enabled = true
rest_bind = "127.0.0.1:7700"
grpc_enabled = false
grpc_bind = "127.0.0.1:7701"
websocket_enabled = true
auth_token = "${IAD_AUTH_TOKEN}"   # env var reference

[health]
default_check_interval = "30s"
default_timeout = "10s"
default_max_failures = 3

[scheduler]
missed_fire_policy = "run-now"    # run-now | skip | queue
timezone = "Asia/Jakarta"

[metrics]
enabled = true
bind = "127.0.0.1:7702"
path = "/metrics"
```

## 2. Data Models

### 2.1 Agent Record

```sql
CREATE TABLE agents (
    id          TEXT PRIMARY KEY,            -- ULID
    name        TEXT NOT NULL UNIQUE,
    version     TEXT NOT NULL,
    runtime     TEXT NOT NULL,               -- python3, node, rust, wasm
    entrypoint  TEXT NOT NULL,
    manifest    TEXT NOT NULL,               -- Full YAML (JSON stored)
    status      TEXT NOT NULL DEFAULT 'registered',
                -- registered | starting | running | stopping | stopped | failed | crashed | disabled
    pid         INTEGER,
    generation_id INTEGER NOT NULL DEFAULT 1, -- Incremented on each restart (stale result detection)
    cgroup_path TEXT,                        -- cgroups v2 scope path (Phase 4+)
    last_exit_code INTEGER,                  -- Exit code from last termination
    last_exit_signal INTEGER,                -- Signal that killed agent (9=SIGKILL, etc.)
    oom_killed  INTEGER DEFAULT 0,           -- 1 if last death was OOM
    started_at  TEXT,                        -- ISO-8601
    stopped_at  TEXT,
    failure_count INTEGER DEFAULT 0,
    last_health TEXT,                        -- ISO-8601
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_name ON agents(name);
```

### 2.2 Task Record

```sql
CREATE TABLE tasks (
    id              TEXT PRIMARY KEY,        -- ULID
    title           TEXT,
    payload         TEXT NOT NULL,            -- JSON
    capabilities    TEXT NOT NULL,            -- JSON array
    priority        INTEGER DEFAULT 2,       -- 0=none, 1=urgent, 2=high, 3=medium, 4=low
    status          TEXT NOT NULL DEFAULT 'pending',
                    -- pending | queued | assigned | leased | running | completed
                    -- | failed | cancelled | cancelling | dead_letter
    assigned_agent  TEXT REFERENCES agents(id),
    result          TEXT,                     -- JSON
    error           TEXT,
    error_class     TEXT,                     -- transient | fatal | budget | timeout | loop
    retry_count     INTEGER DEFAULT 0,
    max_retries     INTEGER DEFAULT 3,
    timeout_ms      INTEGER DEFAULT 300000,  -- 5 min default
    lease_token     TEXT,                     -- Cryptographic token for lease validation
    lease_expires_at TEXT,                    -- ISO-8601, NULL if not leased
    agent_generation INTEGER,                -- generation_id of agent when assigned
    tokens_consumed INTEGER DEFAULT 0,       -- LLM tokens used (in + out)
    cost_usd        REAL DEFAULT 0.0,        -- Accumulated cost for this task
    submitted_at    TEXT NOT NULL DEFAULT (datetime('now')),
    started_at      TEXT,
    completed_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_tasks_agent ON tasks(assigned_agent);
CREATE INDEX idx_tasks_lease ON tasks(status, lease_expires_at)
    WHERE status IN ('leased', 'running');
```

### 2.3 Event Record

```sql
CREATE TABLE events (
    id          TEXT PRIMARY KEY,             -- ULID
    topic       TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    payload     TEXT NOT NULL,                -- JSON
    source      TEXT NOT NULL,                -- agent ID or "daemon"
    sequence    INTEGER NOT NULL,             -- monotonic per topic
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_events_topic ON events(topic, sequence);
CREATE INDEX idx_events_source ON events(source);
```

### 2.4 Dead Letter Queue

```sql
CREATE TABLE dead_letter_tasks (
    id              TEXT PRIMARY KEY,
    original_task_id TEXT NOT NULL REFERENCES tasks(id),
    agent_id        TEXT REFERENCES agents(id),
    failure_reason  TEXT NOT NULL,
    error_class     TEXT NOT NULL,        -- transient | fatal | budget | timeout | loop
    original_payload TEXT NOT NULL,
    partial_result  TEXT,
    retry_count     INTEGER NOT NULL,
    dead_lettered_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 2.5 Agent Incidents (Flight Recorder)

```sql
CREATE TABLE agent_incidents (
    id              TEXT PRIMARY KEY,     -- ULID
    agent_id        TEXT NOT NULL REFERENCES agents(id),
    incident_type   TEXT NOT NULL,        -- oom | timeout | crash | fd_exhaust
                                          -- | circuit_open | cost_limit | tool_loop
                                          -- | context_overflow | network_error
    severity        TEXT NOT NULL,        -- critical | warning | info
    exit_code       INTEGER,
    signal          INTEGER,
    stderr_tail     TEXT,                 -- Last 4KB of stderr
    context         TEXT,                 -- JSON: additional diagnostic data
    resolved_at     TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_incidents_agent ON agent_incidents(agent_id, created_at);
CREATE INDEX idx_incidents_type ON agent_incidents(incident_type);
```

### 2.6 Agent Usage Tracking

```sql
CREATE TABLE agent_usage (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id        TEXT NOT NULL REFERENCES agents(id),
    task_id         TEXT REFERENCES tasks(id),
    tokens_in       INTEGER NOT NULL DEFAULT 0,
    tokens_out      INTEGER NOT NULL DEFAULT 0,
    cost_usd        REAL NOT NULL DEFAULT 0.0,
    tool_calls      INTEGER NOT NULL DEFAULT 0,
    recorded_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_usage_agent_time ON agent_usage(agent_id, recorded_at);
```

### 2.7 Health Tick History

```sql
CREATE TABLE agent_health_ticks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id        TEXT NOT NULL REFERENCES agents(id),
    check_type      TEXT NOT NULL,        -- ping | readiness | resource
    latency_ms      INTEGER NOT NULL,
    status          TEXT NOT NULL,        -- healthy | degraded | unhealthy | timeout
    details         TEXT,                 -- JSON: resource snapshot
    recorded_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_health_cleanup ON agent_health_ticks(recorded_at);
```

### 2.8 Schedule Record

```sql
CREATE TABLE schedules (
    id              TEXT PRIMARY KEY,        -- ULID
    name            TEXT NOT NULL UNIQUE,
    cron_expr       TEXT,                    -- cron expression (optional)
    run_at          TEXT,                    -- one-shot ISO-8601 (optional)
    task_template   TEXT NOT NULL,           -- JSON: task to submit
    enabled         INTEGER DEFAULT 1,
    last_run        TEXT,
    next_run        TEXT,
    run_count       INTEGER DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_schedules_next ON schedules(next_run) WHERE enabled = 1;
```

## 3. API Specification

### 3.1 REST API (v1)

**Base URL:** `http://127.0.0.1:7700/api/v1`

#### Agents

| Method | Path | Description |
|--------|------|-------------|
| GET | `/agents` | List all agents |
| POST | `/agents` | Register agent (YAML/JSON body) |
| GET | `/agents/:id` | Get agent detail |
| PUT | `/agents/:id` | Update agent manifest |
| DELETE | `/agents/:id` | Deregister agent |
| POST | `/agents/:id/start` | Start agent |
| POST | `/agents/:id/stop` | Stop agent |
| POST | `/agents/:id/restart` | Restart agent |
| GET | `/agents/:id/logs` | Get agent logs |
| GET | `/agents/:id/health` | Get health status |

#### Tasks

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tasks` | List tasks (filterable) |
| POST | `/tasks` | Submit task |
| GET | `/tasks/:id` | Get task detail |
| POST | `/tasks/:id/cancel` | Cancel task |

#### Events

| Method | Path | Description |
|--------|------|-------------|
| POST | `/events/publish` | Publish event |
| GET | `/events/:topic` | Read events from topic |
| POST | `/events/subscribe` | Subscribe (returns WS URL) |

#### Schedules

| Method | Path | Description |
|--------|------|-------------|
| GET | `/schedules` | List schedules |
| POST | `/schedules` | Create schedule |
| PUT | `/schedules/:id` | Update schedule |
| DELETE | `/schedules/:id` | Delete schedule |
| POST | `/schedules/:id/trigger` | Trigger now |

#### System

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Daemon health |
| GET | `/status` | Daemon status + stats |
| POST | `/reload` | Hot-reload config |
| POST | `/shutdown` | Graceful shutdown |

### 3.2 WebSocket API

**Endpoint:** `ws://127.0.0.1:7700/ws`

```json
// Subscribe to events
{ "type": "subscribe", "topics": ["agent.*", "task.completed"] }

// Event delivery
{ "type": "event", "topic": "agent.started", "payload": { "agent_id": "...", "name": "content-writer" } }

// Command
{ "type": "command", "action": "agent.restart", "target": "content-writer" }
```

### 3.3 CLI Specification

```bash
# Daemon
iad init                        # Initialize IAD_HOME
iad start                       # Start daemon (daemonize)
iad start --foreground          # Start in foreground
iad stop                        # Graceful stop
iad status                      # Daemon status
iad reload                      # Hot-reload config
iad logs                        # Daemon logs (follow)

# Agents
iad agent add <manifest.yaml>   # Register agent
iad agent remove <name>         # Deregister
iad agent list                  # List agents (table)
iad agent start <name>          # Start agent
iad agent stop <name>           # Stop agent
iad agent restart <name>        # Restart agent
iad agent logs <name>           # Agent logs (follow)
iad agent status <name>         # Agent detail
iad agent inspect <name>        # Full manifest + state

# Tasks
iad task submit <json>          # Submit task
iad task list                   # List tasks
iad task status <id>            # Task detail
iad task cancel <id>            # Cancel task

# Schedules
iad schedule add <name> --cron "*/5 * * * *" --task <json>
iad schedule list
iad schedule remove <name>
iad schedule trigger <name>     # Run now

# Events
iad event publish <topic> <json>
iad event subscribe <topic>     # Stream to stdout
iad event history <topic>       # Replay events
```

## 4. Agent Communication Protocol

### 4.1 Agent SDK Interface

Agents communicate with the daemon via Unix socket. The SDK wraps this.

```python
# Python SDK example
from iad_sdk import AgentClient

client = AgentClient()  # Auto-discovers socket

# Register capabilities
client.register(capabilities=["text-generation", "summarization"])

# Receive tasks
@client.on_task
async def handle_task(task):
    result = await generate_text(task.payload["prompt"])
    return {"text": result}

# Publish event
await client.publish("content.generated", {"article_id": "123"})

# Subscribe to events
@client.on_event("data.updated")
async def on_data_updated(event):
    await refresh_cache(event.payload)

# Health check
@client.health_check
async def check():
    return {"status": "healthy", "model_loaded": True}

client.run()
```

### 4.2 Wire Protocol (Unix Socket)

```
Frame: [4 bytes length (big-endian u32)][payload JSON]
Max frame size: 16 MB (reject with error frame if exceeded)
Read timeout: 30s per frame
Write timeout: 10s per frame

→ Agent to Daemon:
  { "type": "register", "capabilities": [...] }
  { "type": "task_result", "task_id": "...", "lease_token": "...", "result": {...} }
  { "type": "task_progress", "task_id": "...", "tokens_so_far": 450 }
  { "type": "heartbeat_lease", "task_id": "...", "lease_token": "..." }
  { "type": "usage_report", "task_id": "...", "tokens_in": 1200, "tokens_out": 800, "cost_usd": 0.05 }
  { "type": "error", "task_id": "...", "error": "...", "retryable": false, "class": "fatal" }
  { "type": "publish", "topic": "...", "payload": {...} }
  { "type": "pong" }

← Daemon to Agent:
  { "type": "task", "id": "...", "lease_token": "...", "payload": {...} }
  { "type": "cancel_task", "task_id": "..." }
  { "type": "event", "topic": "...", "payload": {...} }
  { "type": "ping" }
  { "type": "shutdown", "grace_period_ms": 5000 }
```

## 5. Error Handling

> **Full fault handling specification:** See [RESILIENCE.md](RESILIENCE.md) for
> comprehensive coverage of all failure modes including process faults, LLM-specific
> failures, network issues, and chaos testing strategies.

### 5.1 Agent Failure Recovery

```
Agent crashes → SIGCHLD received → waitpid() reaps zombie
  → Record exit_code, signal, oom_killed in agents table
  → Log incident to agent_incidents table
  → Check restart_policy:
     - "always": restart (with backoff)
     - "on-failure": restart if exit code != 0
     - "never": mark as stopped
  → Increment generation_id on restart
  → If max_failures exceeded:
     - Mark agent as "failed"
     - Open circuit breaker (stop routing tasks)
     - Send alert (webhook/email)
  → Exponential backoff with jitter: 1s, 2s, 4s, 8s, 16s, 30s (cap), ±25% jitter
  → Global retry budget: max 100 retries/sec across all agents
  → Kill entire process group on stop: kill(-pgid, SIGTERM), then SIGKILL after 5s
```

### 5.2 Daemon Crash Recovery

```
Daemon starts → Acquire PID file flock (LOCK_EX|LOCK_NB):
  → Lock acquired? Proceed
  → Lock failed? Abort (another instance running)
→ SQLite WAL integrity check
→ Load agents with status IN ('running', 'starting', 'crashed')
→ For each:
  → Check if PID alive AND pidfd valid
  → If alive: re-attach monitoring, verify generation_id
  → If dead: set status='crashed', evaluate restart_policy
→ Staggered restart: max 5 concurrent, random jitter, 3s between batches
→ Reclaim expired task leases (lease_expires_at < now)
  → If retry_count < max_retries: reset to 'pending'
  → Else: move to dead_letter_tasks
→ Resume scheduler (evaluate missed_fire_policy for missed jobs)
→ Replay undelivered events from sequence gaps
→ Emit event: daemon.recovered { recovered_agents, reclaimed_tasks }
```

### 5.3 Error Classification

```
Retryable (transient):     Network timeout, HTTP 429/500/502/503, SQLITE_BUSY
Non-retryable (fatal):     HTTP 400/401/403, context overflow, invalid input
Budget exceeded:           Token limit, cost limit → isolate agent
Timeout:                   Task lease expired, stream idle timeout
Loop detected:             Same tool+args called 3+ times → auto-cancel
```

### 5.4 LLM-Specific Failure Handling

```
Mid-stream disconnect:
  → Detect via stream_idle_timeout (30s no new tokens)
  → Cancel agent's upstream HTTP request via CancellationToken
  → Record partial result + tokens consumed
  → Evaluate retry based on error class

Cost runaway:
  → Agent reports usage via usage_report frame
  → Daemon tracks rolling 1h + 24h windows
  → 80% of budget: warn via alert
  → 100% of budget: isolate agent, reject all new tasks

Hallucination/tool loop:
  → Track tool call chain depth + identical call detection
  → max_tool_calls_per_task exceeded → TOOL_LOOP_DETECTED → dead-letter
```

### 5.5 Disk & Storage Fault Handling

```
Disk space < 10%: aggressive data retention cleanup
Disk space < 5%:  stop accepting new tasks, force WAL checkpoint, alert
Disk space < 2%:  read-only mode, refuse all writes, critical alert

SQLite WAL > 100MB: force checkpoint, alert
SQLite BUSY timeout: 5s retry, then propagate error to caller
```

## 6. Security Model

### 6.1 Authentication

```
CLI → Unix socket (uid/gid check — same user)
REST → Bearer token (IAD_AUTH_TOKEN)
gRPC → mTLS (client certificates)
```

### 6.2 Agent Isolation

```
Level 1 (default): Process isolation (separate PID, separate UID optional)
Level 2 (cgroups): Memory/CPU limits via cgroups v2
Level 3 (WASM):    Sandboxed execution, no filesystem access
```

### 6.3 Secret Management

```
Secrets stored in $IAD_HOME/secrets/.vault (encrypted at rest)
Injected as environment variables to agent processes
Never logged, never in manifests plaintext
Key derivation: PBKDF2 from master password or system keyring
```

## 7. Performance Targets

| Operation | Target Latency | Throughput |
|-----------|---------------|------------|
| Agent start | < 200ms | N/A |
| Agent stop (graceful) | < 5s | N/A |
| Task submit → route | < 10ms | 1000/s |
| Event publish → deliver | < 5ms | 5000/s |
| Health check round | < 100ms | N/A |
| CLI command (local) | < 50ms | N/A |
| REST API response | < 20ms | 500 req/s |
| SQLite write | < 1ms | 10000/s |

## 8. Testing Strategy

| Level | Tool | Coverage Target |
|-------|------|----------------|
| Unit | `cargo test` | 80%+ |
| Integration | Custom test harness | All API endpoints |
| E2E | Shell scripts + Python | Full lifecycle scenarios |
| Load | `wrk` / `k6` | Performance targets met |
| Fuzz | `cargo-fuzz` | Wire protocol, API inputs |
| Security | `cargo-audit` + manual | No known CVEs |

---

*Document maintained by Rizoa. Updated with each spec change.*
