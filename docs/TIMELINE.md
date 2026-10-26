# Timeline & Milestones

**Product:** IKAMAI Agent Daemon (IAD)  
**Date:** 2026-10-26  
**Author:** Rizoa  
**Document ID:** IAD-TL-001

---

## 1. Gantt Chart Overview

```
2026                                                           2027
Oct        Nov        Dec        Jan        Feb        Mar        Apr        May        Jun        Jul
 │          │          │          │          │          │          │          │          │          │
 ├──── P0 ──┤          │          │          │          │          │          │          │          │
 │Foundation│          │          │          │          │          │          │          │          │
 │ ████████ │          │          │          │          │          │          │          │          │
 │          │          │          │          │          │          │          │          │          │
 │     ├──────── P1 ──────────┤  │          │          │          │          │          │          │
 │     │  Core Daemon         │  │          │          │          │          │          │          │
 │     │  ████████████████████│  │          │          │          │          │          │          │
 │     │          │          │  │          │          │          │          │          │          │
 │     │          ├──────── P2 ──────────┤  │          │          │          │          │          │
 │     │          │  Tasks & Events      │  │          │          │          │          │          │
 │     │          │  ████████████████████│  │          │          │          │          │          │
 │     │          │          │          │  │          │          │          │          │          │
 │     │          │          │     ├──────────── P3 ──────────────────┤     │          │          │
 │     │          │          │     │  API & Monitoring                │     │          │          │
 │     │          │          │     │  ████████████████████████████████│     │          │          │
 │     │          │          │     │          │          │          │     │          │          │
 │     │          │          │     │          │     ├──────────── P4 ──────────────────┤          │
 │     │          │          │     │          │     │  Scale & Plugins                │          │
 │     │          │          │     │          │     │  ████████████████████████████████│          │
 │     │          │          │     │          │     │          │          │          │          │
 │     │          │          │     │          │     │          │     ├──────────── P5 ──────────┤
 │     │          │          │     │          │     │          │     │  Production & Launch     │
 │     │          │          │     │          │     │          │     │  ████████████████████████│
 │     │          │          │     │          │     │          │     │          │          │
 ▼     ▼          ▼          ▼     ▼          ▼     ▼          ▼     ▼          ▼          ▼
      v0.0.1                v0.1.0          v0.2.0          v0.3.0          v0.5.0       v1.0.0
      scaffold              alpha.1         alpha.2         beta.1          beta.2       GA 🚀
```

---

## 2. Detailed Timeline

### Week-by-Week Breakdown

| Week | Dates | Phase | Key Activities | Deliverable |
|------|-------|-------|---------------|-------------|
| W1 | Oct 27 – Nov 2 | P0 | Scaffold, CI/CD, docs | v0.0.1 |
| W2 | Nov 3 – Nov 9 | P1 | Daemon process, PID, signals | — |
| W3 | Nov 10 – Nov 16 | P1 | Config loading, SQLite storage | — |
| W4 | Nov 17 – Nov 23 | P1 | Agent manifest, registry | — |
| W5 | Nov 24 – Nov 30 | P1 | Process manager, lifecycle, health | **v0.1.0-alpha.1** |
| W6 | Dec 1 – Dec 7 | P1→P2 | CLI polish, unit tests | — |
| W7 | Dec 8 – Dec 14 | P2 | IPC protocol, SDK wire format | — |
| W8 | Dec 15 – Dec 21 | P2 | Task model, queue, routing | — |
| W9 | Dec 22 – Dec 28 | P2 | Event bus, scheduler | — |
| W10 | Dec 29 – Jan 4 | P2 | Python SDK, integration tests | **v0.2.0-alpha.2** |
| W11-12 | Jan 5 – Jan 18 | P3 | REST API server (Axum) | — |
| W13-14 | Jan 19 – Feb 1 | P3 | Agent/Task/Event API endpoints | — |
| W15-16 | Feb 2 – Feb 15 | P3 | WebSocket, auth, metrics | — |
| W17-18 | Feb 16 – Feb 28 | P3 | Alerts, hot-reload, API docs, Node SDK | **v0.3.0-beta.1** |
| W19-20 | Mar 1 – Mar 15 | P4 | gRPC server, proto definitions | — |
| W21-22 | Mar 16 – Mar 29 | P4 | Inter-daemon comm, node discovery | — |
| W23-24 | Mar 30 – Apr 12 | P4 | WASM runtime, PostgreSQL backend | — |
| W25-26 | Apr 13 – Apr 30 | P4 | mTLS, RBAC, secrets, tests | **v0.5.0-beta.2** |
| W27-28 | May 1 – May 15 | P5 | Documentation suite | — |
| W29-30 | May 16 – May 31 | P5 | Benchmarks, security audit | — |
| W31-32 | Jun 1 – Jun 15 | P5 | Docker, examples, RC | **v0.9.0-rc.1** |
| W33-34 | Jun 16 – Jun 30 | P5 | Final polish, launch | **v1.0.0 GA 🚀** |

---

## 3. Milestones Detail

### M1: First Heartbeat — v0.1.0-alpha.1 (Nov 30, 2026)

**Acceptance Criteria:**
- [ ] `iad init` creates `~/.iad/` directory structure
- [ ] `iad start` starts daemon as background process
- [ ] `iad start --foreground` starts in foreground with structured logs
- [ ] `iad stop` sends graceful shutdown signal
- [ ] `iad status` shows daemon running/stopped
- [ ] `iad agent add <manifest.yaml>` registers agent
- [ ] `iad agent list` shows registered agents with status
- [ ] `iad agent start <name>` starts agent process
- [ ] `iad agent stop <name>` stops agent gracefully
- [ ] Health check detects agent failure and auto-restarts
- [ ] SQLite persists state across daemon restarts
- [ ] Structured JSON logging to stdout and file
- [ ] CI green (fmt, clippy, test, audit)

**Demo Scenario:**
```bash
iad init
iad start
iad agent add examples/echo-agent/manifest.yaml
iad agent list    # shows echo-agent: running
kill <agent_pid>  # daemon auto-restarts it
iad agent list    # shows echo-agent: running (restarted)
iad stop
```

---

### M2: Agents Talk — v0.2.0-alpha.2 (Dec 31, 2026)

**Acceptance Criteria:**
- [ ] IPC protocol over Unix sockets (length-prefixed JSON)
- [ ] Agent can register capabilities via SDK
- [ ] `iad task submit '{"capabilities": ["echo"], "payload": {...}}'` works
- [ ] Task routed to agent matching capabilities
- [ ] Task result returned and stored
- [ ] Task retry on failure with exponential backoff
- [ ] Event bus: agent publishes event, subscriber receives
- [ ] Cron scheduler: create schedule, verify periodic execution
- [ ] Python SDK: basic agent registration and task handling

---

### M3: Control Plane — v0.3.0-beta.1 (Feb 28, 2027)

**Acceptance Criteria:**
- [ ] REST API at `http://localhost:7700/api/v1/*`
- [ ] All operations available via API (agents, tasks, schedules, events)
- [ ] WebSocket at `ws://localhost:7700/ws` for real-time events
- [ ] Bearer token authentication for API
- [ ] Prometheus metrics at `:7702/metrics`
- [ ] Webhook alerts on agent/task failure
- [ ] Hot-reload config without restart
- [ ] OpenAPI spec generated
- [ ] Node.js SDK basic functionality

---

### M4: Fleet Ready — v0.5.0-beta.2 (Apr 30, 2027)

**Acceptance Criteria:**
- [ ] gRPC inter-daemon communication
- [ ] 3-node fleet with cross-node task routing
- [ ] WASM agent runs in sandbox
- [ ] PostgreSQL backend works as alternative to SQLite
- [ ] mTLS for inter-daemon gRPC
- [ ] RBAC: admin vs operator vs viewer roles
- [ ] Encrypted secret vault
- [ ] Resource limits via cgroups v2

---

### M5: Production Launch — v1.0.0 (Jun 30, 2027)

**Acceptance Criteria:**
- [ ] All P0 + P1 requirements from PRD met
- [ ] Benchmark: 100 agents, <50MB daemon overhead
- [ ] Security audit passed (0 critical, 0 high)
- [ ] Docker image + docker-compose.yml
- [ ] systemd unit file
- [ ] 5 example agents (echo, Python scraper, Node API, Rust processor, WASM plugin)
- [ ] Complete documentation (quickstart, agent-dev, deployment, API)
- [ ] Landing page live
- [ ] GitHub Release with binaries for 4 platforms
- [ ] Blog post + social media announcement

---

## 4. Risk Register

| Risk | Impact | Probability | Mitigation | Owner |
|------|--------|-------------|------------|-------|
| Rust learning curve delays Phase 1 | 2-3 week slip | Medium | Start with simple modules, iterate | Rizoa |
| Scope creep in Phase 2 (SDK) | 2-4 week slip | High | Strict MVP scope, defer nice-to-haves | Rizoa |
| gRPC complexity in Phase 4 | 3-4 week slip | Medium | Prototype early, use tonic examples | Rizoa |
| WASM plugin API design | 2-3 week slip | Medium | Study wasmtime examples, start minimal | Rizoa |
| Security audit findings | 2-4 week delay for GA | Low | Run cargo-audit in CI from day 1 | Rizoa |
| Burnout (solo developer) | Entire project at risk | Medium | Maintain sustainable pace, 20h/week max | Rizoa |

---

## 5. Sprint Planning Template

```
Sprint N: [Phase X — Focus Area]
Duration: 2 weeks (Mon-Fri)
Capacity: 40 hours

Goals:
  1. [Primary goal]
  2. [Secondary goal]
  3. [Stretch goal]

Tasks:
  - [ ] Task 1 (8h) — IKA-XXX
  - [ ] Task 2 (6h) — IKA-XXX
  - [ ] Task 3 (4h) — IKA-XXX
  - [ ] Tests (8h)
  - [ ] Docs (4h)
  Total: 30h (75% capacity — leave buffer)

Done when:
  - All tasks checked
  - CI green
  - CHANGELOG updated
  - Sprint retro notes written
```

---

*Timeline maintained by Rizoa. Updated weekly.*
