# Technical Specification

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-26  
**Author:** Rizoa  
**Status:** DRAFT

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
                -- registered | starting | running | stopping | stopped | failed | disabled
    pid         INTEGER,
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
                    -- pending | queued | assigned | running | completed | failed | cancelled
    assigned_agent  TEXT REFERENCES agents(id),
    result          TEXT,                     -- JSON
    error           TEXT,
    retry_count     INTEGER DEFAULT 0,
    max_retries     INTEGER DEFAULT 3,
    timeout_ms      INTEGER DEFAULT 300000,  -- 5 min default
    submitted_at    TEXT NOT NULL DEFAULT (datetime('now')),
    started_at      TEXT,
    completed_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_tasks_agent ON tasks(assigned_agent);
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

### 2.4 Schedule Record

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
Frame: [4 bytes length][payload JSON]

→ Agent to Daemon:
  { "type": "register", "capabilities": [...] }
  { "type": "task_result", "task_id": "...", "result": {...} }
  { "type": "publish", "topic": "...", "payload": {...} }
  { "type": "heartbeat" }

← Daemon to Agent:
  { "type": "task", "id": "...", "payload": {...} }
  { "type": "event", "topic": "...", "payload": {...} }
  { "type": "health_check" }
  { "type": "shutdown" }
```

## 5. Error Handling

### 5.1 Agent Failure Recovery

```
Agent crashes → Health Monitor detects (heartbeat timeout)
  → Check restart_policy:
     - "always": restart immediately
     - "on-failure": restart if exit code != 0
     - "never": mark as stopped
  → If max_failures exceeded:
     - Mark agent as "failed"
     - Send alert (webhook/email)
     - Circuit breaker: stop routing tasks
  → Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s (max)
```

### 5.2 Daemon Crash Recovery

```
Daemon starts → Check PID file:
  → Stale PID? Remove and continue
  → Active PID? Abort (already running)
→ Load state from SQLite
→ Resume agents that were "running":
  → Check if PID still alive
  → If alive: re-attach monitoring
  → If dead: restart per policy
→ Resume scheduled jobs
→ Replay undelivered events
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
