# Master Plan

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-27  
**Author:** Rizoa  
**Status:** ACTIVE  
**See also:** [RESILIENCE.md](RESILIENCE.md) — Comprehensive fault handling specification

---

## 1. Phase Overview

```
Phase 0: Foundation         [Month 1]      ███░░░░░░░  
Phase 1: Core Daemon        [Month 1-2]    █████░░░░░  
Phase 2: Task & Events      [Month 2-3]    ███████░░░  
Phase 3: API & Monitoring   [Month 3-5]    ████████░░  
Phase 4: Scale & Plugins    [Month 5-7]    █████████░  
Phase 5: Production         [Month 7-9]    ██████████  
Phase 6: Ecosystem          [Month 9+]     ██████████+
```

## 2. Phase Details

### Phase 0: Foundation (Month 1, Week 1-2)

**Goal:** Project scaffolding, CI/CD, documentation foundation.

| # | Task | Estimate | Status |
|---|------|----------|--------|
| 0.1 | Create GitHub repo + branch protection | 1h | ⬜ |
| 0.2 | Scaffold Rust workspace (cargo init) | 2h | ⬜ |
| 0.3 | Setup CI/CD (GitHub Actions: fmt, clippy, test) | 3h | ⬜ |
| 0.4 | Write WHITEPAPER, PRD, SPEC, PLAN docs | 8h | ✅ |
| 0.5 | Setup Linear project + import tasks | 2h | ⬜ |
| 0.6 | Define Cargo workspace structure | 2h | ⬜ |
| 0.7 | Setup pre-commit hooks (fmt, clippy) | 1h | ⬜ |
| 0.8 | Create initial README with badges | 1h | ⬜ |

**Deliverable:** Empty but fully-tooled project, all docs, CI green.

### Phase 1: Core Daemon (Month 1-2, Week 2-6)

**Goal:** A daemon that starts, manages agent processes, and stays running.

| # | Task | Estimate | Depends On |
|---|------|----------|------------|
| 1.1 | Daemon process (daemonize, PID file **with flock**, signal handling) | 8h | 0.6 |
| 1.2 | Configuration loading (TOML parser + env vars) | 4h | 0.6 |
| 1.3 | SQLite storage layer (migrations, connection pool, **WAL + integrity check**) | 6h | 0.6 |
| 1.4 | Agent manifest parser (YAML → struct) | 4h | 1.2 |
| 1.5 | Agent registry (register, deregister, list, **generation_id**) | 6h | 1.3, 1.4 |
| 1.6 | Process manager (spawn, kill, signal, **process group, PR_SET_PDEATHSIG**) | 10h | 1.1 |
| 1.7 | Agent lifecycle (start → running → stop, **crashed state**) | 8h | 1.5, 1.6 |
| 1.8 | Health monitor (heartbeat, failure detection) | 6h | 1.7 |
| 1.9 | Auto-restart with **exponential backoff + jitter + staggered restart** | 6h | 1.8 |
| 1.10 | Structured logging (tracing crate, **fault event logging**) | 4h | 1.1 |
| 1.11 | CLI: `iad init/start/stop/status` | 6h | 1.1, 1.2 |
| 1.12 | CLI: `iad agent add/remove/list/start/stop/logs` | 8h | 1.5, 1.7 |
| 1.R1 | **Zombie reaping**: SIGCHLD handler + waitpid loop | 4h | 1.6 |
| 1.R2 | **Orphan cleanup**: process group kill (-pgid), PR_SET_PDEATHSIG | 3h | 1.6 |
| 1.R3 | **FD leak prevention**: CLOEXEC, AgentProcess owns FDs, self-monitoring | 4h | 1.6 |
| 1.R4 | **Crash recovery**: startup recovery sequence (adopt/restart agents) | 6h | 1.7 |
| 1.R5 | **Agent incidents table**: flight recorder for crash diagnostics | 3h | 1.3 |
| 1.13 | Unit tests for all Phase 1 modules | 8h | 1.1-1.R5 |
| 1.14 | Integration test: full agent lifecycle + **crash/restart test** | 6h | 1.13 |

**Deliverable:** `iad start`, register agents, auto-restart on failure, **crash-safe process management**. → **v0.1.0-alpha.1**

### Phase 2: Task Routing & Event Bus (Month 2-3, Week 6-10)

**Goal:** Agents can receive tasks and communicate via events.

| # | Task | Estimate | Depends On |
|---|------|----------|------------|
| 2.1 | Agent-daemon IPC protocol (Unix socket, **frame timeouts, max size**) | 8h | 1.7 |
| 2.2 | Agent SDK wire protocol (length-prefixed JSON, **PING/PONG, error class**) | 8h | 2.1 |
| 2.3 | Task model + storage (**lease_token, error_class, dead_letter**) | 6h | 1.3 |
| 2.4 | Task submission + **bounded queue + backpressure (HTTP 429)** | 6h | 2.3 |
| 2.5 | Capability-based task routing | 6h | 2.4, 1.5 |
| 2.6 | Task lifecycle (pending → leased → running → done, **lease expiry**) | 8h | 2.5 |
| 2.7 | Task retry + timeout + cancellation + **error classification** | 8h | 2.6 |
| 2.8 | Event bus: publish/subscribe | 8h | 2.1 |
| 2.9 | Event persistence + replay | 4h | 2.8, 1.3 |
| 2.10 | Scheduler: cron + one-shot | 6h | 2.4, 1.3 |
| 2.11 | CLI: `iad task submit/list/status/cancel` + **`dead-letter list/retry`** | 6h | 2.4-2.7 |
| 2.12 | CLI: `iad schedule add/list/remove/trigger` | 4h | 2.10 |
| 2.13 | Python SDK (basic) | 8h | 2.2 |
| 2.R1 | **Retry storm prevention**: global retry budget + per-agent circuit breaker | 6h | 2.7 |
| 2.R2 | **Task lease system**: lease_token, TTL, heartbeat_lease, reclaim | 6h | 2.6 |
| 2.R3 | **Cancellation propagation**: CANCEL_TASK frame, CancellationToken in SDK | 4h | 2.2, 2.7 |
| 2.R4 | **Usage tracking**: agent_usage table, token/cost reporting frames | 4h | 2.2, 1.3 |
| 2.14 | Tests for Phase 2 + **chaos tests (slowloris, retry storm)** | 10h | all |

**Deliverable:** Tasks route to agents, events flow, scheduler works, **resilient under faults**. → **v0.2.0-alpha.2**

### Phase 3: API & Monitoring (Month 3-5, Week 10-18)

**Goal:** REST API, WebSocket, metrics, alerting.

| # | Task | Estimate | Depends On |
|---|------|----------|------------|
| 3.1 | REST API server (Axum) | 8h | 1.1 |
| 3.2 | Agent API endpoints | 6h | 3.1, 1.5 |
| 3.3 | Task API endpoints | 6h | 3.1, 2.4 |
| 3.4 | Schedule API endpoints | 4h | 3.1, 2.10 |
| 3.5 | Event API endpoints | 4h | 3.1, 2.8 |
| 3.6 | System API endpoints (health, status, reload) | 4h | 3.1 |
| 3.7 | API authentication (bearer token) | 4h | 3.1 |
| 3.8 | WebSocket server (real-time events) | 8h | 3.1, 2.8 |
| 3.9 | Prometheus metrics endpoint | 6h | 3.1 |
| 3.10 | Alert webhooks (agent failure, task failure) | 4h | 1.8 |
| 3.11 | Hot-reload configuration | 4h | 1.2 |
| 3.12 | API documentation (OpenAPI spec) | 4h | 3.2-3.6 |
| 3.13 | Node.js SDK (basic) | 6h | 2.2 |
| 3.14 | Tests for Phase 3 | 8h | all |
| 3.15 | Load testing (k6) | 4h | 3.1 |

**Deliverable:** Full REST API, WebSocket, metrics. → **v0.3.0-beta.1**

### Phase 4: Scale & Plugins (Month 5-7, Week 18-26)

**Goal:** Multi-node, gRPC, WASM plugins, production hardening.

| # | Task | Estimate | Depends On |
|---|------|----------|------------|
| 4.1 | gRPC server (tonic) | 8h | 1.1 |
| 4.2 | gRPC proto definitions | 4h | 4.1 |
| 4.3 | Inter-daemon communication (gRPC) | 8h | 4.1, 4.2 |
| 4.4 | Node discovery + registration | 6h | 4.3 |
| 4.5 | Cross-node task routing | 8h | 4.4, 2.5 |
| 4.6 | Cross-node event propagation | 6h | 4.4, 2.8 |
| 4.7 | PostgreSQL storage backend | 8h | 1.3 |
| 4.8 | WASM plugin runtime (wasmtime) | 12h | 1.7 |
| 4.9 | WASM agent sandboxing | 8h | 4.8 |
| 4.10 | mTLS for gRPC | 4h | 4.1 |
| 4.11 | RBAC for API | 6h | 3.7 |
| 4.12 | Secret management (encrypted vault) | 6h | 1.2 |
| 4.13 | Resource limits (cgroups v2) | 6h | 1.6 |
| 4.14 | Tests for Phase 4 | 12h | all |

**Deliverable:** Multi-node fleet, WASM plugins. → **v0.5.0-beta.2**

### Phase 5: Production (Month 7-9, Week 26-34)

**Goal:** Production-ready, documented, benchmarked, audited.

| # | Task | Estimate | Depends On |
|---|------|----------|------------|
| 5.1 | Comprehensive documentation | 16h | all |
| 5.2 | Quick-start guide | 4h | 5.1 |
| 5.3 | Agent development guide | 8h | 5.1 |
| 5.4 | Deployment guide (systemd, Docker) | 4h | 5.1 |
| 5.5 | Performance benchmarks | 8h | all |
| 5.6 | Security audit | 12h | all |
| 5.7 | Fuzz testing | 8h | all |
| 5.8 | Docker image + Compose | 4h | all |
| 5.9 | systemd unit file | 2h | all |
| 5.10 | Example agents (3-5) | 8h | 2.13 |
| 5.11 | Logo + branding | 4h | — |
| 5.12 | Website / landing page | 8h | 5.11 |
| 5.13 | Release v1.0.0 | 4h | all |

**Deliverable:** Production-ready release. → **v1.0.0**

### Phase 6: Ecosystem (Month 9+)

**Goal:** Community, integrations, enterprise features.

| # | Task | Priority |
|---|------|----------|
| 6.1 | Web dashboard UI | P1 |
| 6.2 | Agent marketplace | P2 |
| 6.3 | Terraform provider | P2 |
| 6.4 | VS Code extension | P3 |
| 6.5 | Slack/Discord integration | P2 |
| 6.6 | SSO (SAML/OIDC) | P2 |
| 6.7 | Multi-tenancy | P3 |
| 6.8 | Distributed tracing UI | P2 |
| 6.9 | Agent template library | P1 |
| 6.10 | Mobile monitoring app | P3 |

**Deliverable:** Growing ecosystem. → **v1.x, v2.0**

## 3. Resource Allocation

| Role | Allocation | Person |
|------|-----------|--------|
| Lead / Architect | 60% | Rizoa |
| Rust Development | 80% | Rizoa + contributors |
| SDK Development | 20% | Rizoa |
| Testing & QA | 15% | Rizoa + CI |
| Documentation | 15% | Rizoa |
| DevOps / CI | 10% | Rizoa |

## 4. Total Effort Estimate

| Phase | Hours | Weeks | Notes |
|-------|-------|-------|-------|
| Phase 0 | 20h | 2 | |
| Phase 1 | 104h | 5 | +24h resilience (zombie, orphan, FD, crash recovery) |
| Phase 2 | 118h | 6 | +36h resilience (lease, backpressure, retry, cancel) |
| Phase 3 | 96h | 8 | +16h observability, disk monitoring, debug CLI |
| Phase 4 | 108h | 8 | +14h pidfd, cgroups, OOM, network partition |
| Phase 5 | 98h | 8 | +8h chaos test suite, cost guardrails |
| **Total** | **544h** | **37 weeks** | +98h for production resilience |

## 5. Decision Log

| # | Date | Decision | Rationale |
|---|------|----------|-----------|
| D1 | 2026-10-26 | Use Rust for daemon | Performance + safety critical for daemon |
| D2 | 2026-10-26 | SQLite first, Postgres later | Simplify MVP, scale when needed |
| D3 | 2026-10-26 | Unix sockets for local IPC | Zero-copy, lowest latency |
| D4 | 2026-10-26 | WASM for plugin agents | Sandboxed, portable, polyglot |
| D5 | 2026-10-26 | Axum for REST, tonic for gRPC | Same Tokio ecosystem, composable |
| D6 | 2026-10-27 | PID file with flock (not check-then-write) | Prevent TOCTOU race on daemon startup |
| D7 | 2026-10-27 | Process groups for all agents | Reliable orphan cleanup via kill(-pgid) |
| D8 | 2026-10-27 | Task lease system with heartbeat | Detect hung agents, reclaim tasks |
| D9 | 2026-10-27 | Global retry budget (token bucket) | Prevent retry storms across fleet |
| D10 | 2026-10-27 | Error classification (retryable/fatal) | Avoid retrying non-recoverable errors |
| D11 | 2026-10-27 | Agent generation counter | Discard stale results after crash/restart |

---

*Plan maintained by Rizoa. Updated at each phase gate.*
