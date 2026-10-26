# IKAMAI Agent Daemon — Whitepaper

**Version:** 1.0.0  
**Date:** 2026-10-26  
**Author:** Rizoa (IKAMAI)  
**Status:** PUBLISHED  
**Document ID:** IAD-WP-001

---

## Abstract

**IKAMAI Agent Daemon (IAD)** is a persistent, event-driven orchestration daemon that manages, schedules, monitors, and communicates across a fleet of autonomous AI agents. It acts as the central nervous system for AI automation stacks — coordinating agent lifecycle, inter-agent messaging, task routing, health monitoring, and failover — all from a single, always-on process.

IAD is to AI agents what **systemd** is to system services — but with built-in task routing, inter-agent messaging, capability-based scheduling, WASM sandboxing, and full observability. Written in Rust for maximum performance and reliability.

---

## 1. Problem Statement

### 1.1 The Multi-Agent Chaos

Modern AI-powered organizations deploy multiple autonomous agents: coding assistants, data scrapers, trading bots, content generators, customer support bots, DevOps monitors. Each runs in isolation, creating:

- **No unified lifecycle management** — agents crash silently, restart randomly, or drift into zombie states.
- **No inter-agent communication** — agent A cannot signal agent B; coordination requires fragile glue scripts.
- **No centralized scheduling** — cron jobs scattered across systems, servers, and user crontabs.
- **No health visibility** — no single pane of glass; operators `ssh` into machines to check.
- **No policy enforcement** — agents run with unchecked permissions, consume unlimited resources.
- **No audit trail** — no record of what ran, when, why, or what it produced.

### 1.2 Cost of Chaos

| Problem | Business Impact |
|---------|----------------|
| Silent agent crashes | Revenue loss (trading bots), data gaps (scrapers), SLA breaches |
| No IPC | Duplicate work, stale data, inconsistent state |
| Scattered scheduling | Missed jobs, conflicting execution, debugging nightmares |
| No observability | Hours of firefighting, MTTR in days not minutes |
| No resource limits | Runaway agents consuming all CPU/RAM, cascading failures |

### 1.3 Existing Solutions Fall Short

| Solution | Gap |
|----------|-----|
| **Kubernetes** | Overkill for 5-50 agent processes; no AI-native primitives (capabilities, tasks); steep learning curve |
| **Supervisord** | No inter-agent messaging, no event routing, no task queue, no metrics |
| **systemd** | No multi-host, no task queue, no agent awareness, no capability matching |
| **Docker Compose** | No task routing, no IPC, no health-aware scheduling, no event bus |
| **Custom scripts** | Fragile, no observability, no recovery, impossible to maintain at scale |
| **LangGraph/CrewAI** | Framework-level, not daemon-level; tied to specific LLM frameworks; no process isolation |

---

## 2. Vision

IAD provides a **lightweight, AI-native daemon** that:

1. **Manages agent lifecycle** — start, stop, restart, health-check, auto-recover with configurable policies
2. **Routes tasks** — assign work to agents based on capability, load, priority, and affinity
3. **Enables IPC** — pub/sub + RPC between agents (local Unix sockets, remote gRPC)
4. **Schedules jobs** — cron-like scheduling with dependency DAGs and missed-fire policies
5. **Enforces policies** — rate limits, resource budgets, permission scopes, execution quotas
6. **Observes everything** — structured logs, Prometheus metrics, OpenTelemetry traces, audit trail
7. **Scales horizontally** — single-node to multi-node fleet with zero config change
8. **Sandboxes untrusted code** — WASM plugin agents run safely with no filesystem access

### 2.1 Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Zero-Config Start** | `iad init && iad start` — running in 30 seconds |
| **AI-Native Primitives** | First-class: Agent, Capability, Task, Tool, Event — not just "processes" |
| **Convention over Configuration** | Sensible defaults, override when needed |
| **Fail-Safe** | Every component has a failure mode, recovery strategy, and circuit breaker |
| **Observable by Default** | Structured logs, metrics, traces from day one — not bolted on |
| **Polyglot** | Python, Node.js, Go, Rust, Shell, or WASM — daemon manages all uniformly |
| **Event-Sourced** | Full audit trail, replay, time-travel debugging of all state changes |

---

## 3. Architecture

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    IKAMAI Agent Daemon (IAD)                 │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │ Agent    │  │ Task     │  │ Event    │  │ Policy     │  │
│  │ Registry │  │ Router   │  │ Bus      │  │ Engine     │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬──────┘  │
│       │              │              │               │        │
│  ┌────┴──────────────┴──────────────┴───────────────┴────┐  │
│  │                 Core Daemon Process                    │  │
│  │   Event Loop (Tokio) + Scheduler + Health Monitor      │  │
│  └────┬──────────────┬──────────────┬────────────────────┘  │
│       │              │              │                        │
│  ┌────┴─────┐  ┌─────┴────┐  ┌─────┴─────┐                 │
│  │ Agent A  │  │ Agent B  │  │ Agent C   │  ...             │
│  │ (Python) │  │ (Node)   │  │ (WASM)    │                  │
│  └──────────┘  └──────────┘  └───────────┘                  │
│                                                             │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Storage Layer (SQLite / PostgreSQL)        │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌────────────────────────────────────────────────────────┐ │
│  │       API Layer (CLI + REST + gRPC + WebSocket)         │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Core Components

| Component | Responsibility | Crate |
|-----------|---------------|-------|
| **Core Daemon** | Main event loop, signal handling, graceful shutdown | `iad-core` |
| **Agent Registry** | Register/discover agents, capabilities, health status | `iad-core` |
| **Task Router** | Route tasks to capable agents, load balancing, priority queues | `iad-core` |
| **Event Bus** | Pub/sub + request/reply messaging between agents | `iad-core` |
| **Scheduler** | Cron-like job scheduling with dependency DAGs | `iad-core` |
| **Policy Engine** | Rate limiting, resource budgets, permission scopes | `iad-core` |
| **Health Monitor** | Heartbeat checks, auto-restart, circuit breakers | `iad-core` |
| **Storage** | Persistent state, task queue, audit log | `iad-storage` |
| **API Layer** | External control plane (CLI, REST, gRPC, WebSocket) | `iad-api` |
| **SDK** | Agent-side communication library | `iad-sdk` |
| **CLI** | Command-line interface for operators | `iad` |

### 3.3 Agent Model

Each managed agent is defined by a **manifest**:

```yaml
# agent.manifest.yaml
name: "content-writer"
version: "1.2.0"
runtime: "python3.12"
entrypoint: "main.py"
working_dir: "/opt/agents/content-writer"
capabilities:
  - "text-generation"
  - "seo-optimization"
resources:
  max_memory: "512Mi"
  max_cpu: "1.0"
  max_concurrent_tasks: 5
  max_tasks_per_minute: 100
health:
  check_interval: "30s"
  timeout: "10s"
  max_failures: 3
  restart_policy: "on-failure"  # always | on-failure | never
  restart_delay: "5s"
  max_restart_delay: "300s"     # exponential backoff cap
dependencies:
  - "database-agent"             # must be running before this agent starts
env:
  MODEL: "gpt-4o"
  TEMPERATURE: "0.7"
labels:
  team: "content"
  environment: "production"
```

### 3.4 Communication Model

```
Agent A ──publish──► Event Bus ──deliver──► Agent B
                        │
Agent C ──request──►    │    ──reply──► Agent C
                        │
External ──webhook──► API ──route──► Agent D
```

**Protocols:**
- **Local IPC**: Unix domain sockets (zero-copy, low latency)
- **Remote IPC**: gRPC over mTLS (multi-node fleet)
- **External**: REST/WebSocket API (human/tool integration)

### 3.5 Data Flow Architecture

```
┌──────────┐    ┌──────────┐    ┌──────────┐
│ Ingest   │    │ Process  │    │ Output   │
│          │    │          │    │          │
│ CLI      ├───►│ Validate ├───►│ Execute  │
│ REST API ├───►│ Route    ├───►│ Store    │
│ gRPC     ├───►│ Schedule ├───►│ Emit     │
│ WebSocket├───►│ Queue    ├───►│ Alert    │
│ Event    │    │ Policy   │    │ Log      │
└──────────┘    └──────────┘    └──────────┘
       │              │              │
       └──────────────┴──────────────┘
              Event-Sourced Audit Log
              (every mutation recorded)
```

---

## 4. Technology Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------| 
| **Language** | Rust 1.80+ | Performance, memory safety, zero-cost abstractions, fearless concurrency |
| **Async Runtime** | Tokio | Battle-tested async I/O, multi-threaded work-stealing scheduler |
| **IPC (local)** | Unix Domain Sockets | Zero-copy, kernel-optimized, <1μs latency |
| **IPC (remote)** | gRPC (tonic) | Multiplexed, typed, bidirectional streaming |
| **Storage** | SQLite (embedded) + PostgreSQL (scale) | Simple start → scale when needed |
| **Config** | TOML + env vars | Human-friendly + 12-factor compliant |
| **API** | Axum (REST) + tonic (gRPC) | Unified Tokio ecosystem, composable middleware |
| **Observability** | OpenTelemetry + tracing | Vendor-neutral traces/metrics/logs |
| **CLI** | clap 4 | Ergonomic Rust CLI with derive macros |
| **Plugin System** | WASM (wasmtime) | Sandboxed, portable, polyglot agent plugins |
| **Serialization** | serde + serde_json + serde_yaml | Rust's standard serialization ecosystem |
| **ID Generation** | ULID | Sortable, unique, timestamp-embedded |
| **Scheduling** | cron (Rust crate) | Standard cron expression parsing |

---

## 5. Key Differentiators

| # | Differentiator | Description |
|---|---------------|-------------|
| 1 | **AI-Native Primitives** | First-class concepts: Agent, Capability, Task, Tool — not generic "processes" |
| 2 | **Zero-Config Single Node** | `iad init && iad start` — production-ready in 30 seconds |
| 3 | **WASM Plugin Agents** | Run untrusted agent code safely in WASM sandbox — no filesystem, no network |
| 4 | **Capability-Based Routing** | Tasks find the right agent automatically based on declared capabilities |
| 5 | **Built-in Observability** | OpenTelemetry from day one; structured JSON logs, Prometheus metrics, distributed traces |
| 6 | **Polyglot Agents** | Python, Node.js, Go, Rust, Shell, or WASM — daemon manages all uniformly |
| 7 | **Event Sourcing** | Full audit trail, replay capability, time-travel debugging |
| 8 | **Lightweight** | <50MB daemon memory, <5MB per agent overhead, <10ms task routing |
| 9 | **Horizontal Scale** | Single-node to multi-node fleet with zero code change — just add `[cluster]` config |
| 10 | **Crash-Resilient** | Agent crash → auto-restart with backoff; Daemon crash → resume from persisted state |

---

## 6. Use Cases

### 6.1 IKAMAI Internal
- **Coding Agent Orchestration**: Coordinate multiple Pi/Claude Code agents across repositories
- **Trading Bot Fleet**: Manage XAUBot instances with risk management policies and circuit breakers
- **Content Pipeline**: Schedule and chain Ahli News + Adobe Stock generation agents
- **DevOps Monitors**: Manage health-check agents across 8+ production servers
- **Data Pipeline**: Coordinate scraping → processing → storage → notification agents

### 6.2 External / Open Source
- **Multi-Agent AI Applications**: Orchestrate teams of specialized AI agents
- **Microservice Coordination**: Lightweight alternative to K8s for AI-heavy workloads
- **Edge AI Fleet**: Manage AI agents on IoT devices and edge nodes
- **CI/CD Agent Management**: Coordinate build, test, deploy agent workflows
- **Research & Experimentation**: Quickly spin up agent experiments with full observability

---

## 7. Business Model

| Tier | Features | Price |
|------|----------|-------|
| **Community (OSS)** | Core daemon, 10 agents, single node, SQLite, CLI + REST API | Free forever |
| **Pro** | Unlimited agents, multi-node cluster, PostgreSQL, gRPC, WASM plugins, priority support | $29/mo |
| **Enterprise** | SSO/SAML, RBAC, audit logs, SLA, custom plugins, dedicated support, compliance reports | Custom pricing |

### 7.1 Revenue Streams
1. **Pro/Enterprise Subscriptions** — recurring revenue
2. **Agent Marketplace Commission** — 15% on agent template sales
3. **Managed Cloud Offering** — IAD-as-a-Service (planned v2.0+)
4. **Consulting & Integration** — custom deployment and agent development
5. **Training & Certification** — IAD operator and agent developer certification

---

## 8. Roadmap Summary

| Phase | Version | Timeline | Deliverable |
|-------|---------|----------|-------------|
| **Phase 0: Foundation** | — | Month 1 | Scaffold, CI/CD, docs, Linear tracking |
| **Phase 1: Core Daemon** | v0.1.0-alpha.1 | Month 1-2 | Agent lifecycle, CLI, health monitor |
| **Phase 2: Tasks & Events** | v0.2.0-alpha.2 | Month 2-3 | Task routing, event bus, scheduler, Python SDK |
| **Phase 3: API & Monitoring** | v0.3.0-beta.1 | Month 3-5 | REST API, WebSocket, metrics, alerting |
| **Phase 4: Scale & Plugins** | v0.5.0-beta.2 | Month 5-7 | Multi-node, gRPC, WASM, PostgreSQL |
| **Phase 5: Production** | v1.0.0 | Month 7-9 | GA release, docs, benchmarks, security audit |
| **Phase 6: Ecosystem** | v1.x+ | Month 9+ | Dashboard, marketplace, enterprise features |

---

## 9. Success Metrics

| Metric | Target (v1.0) | Measurement |
|--------|---------------|-------------|
| Agent restart time | < 500ms | Benchmark suite |
| Task routing latency | < 10ms (local) | p99 under load |
| Event delivery latency | < 5ms (local) | p99 under load |
| Max concurrent agents | 100+ (single node) | Soak test (1 hour) |
| Daemon uptime | 99.9% | Production monitoring |
| Memory overhead per agent | < 5MB | heaptrack profiling |
| Daemon base memory | < 50MB | heaptrack profiling |
| CLI command latency | < 50ms | Benchmark suite |
| REST API p99 latency | < 20ms | k6 load test |
| GitHub stars (6 months) | 500+ | Community metric |

---

## 10. Competitive Landscape

```
                    Lightweight ←──────────────────→ Heavyweight
                         │                              │
              IAD ◄──────┤                              │
              systemd ◄──┤                              │
              supervisord◄┤                              │
                         │         Docker Swarm ◄───────┤
                         │                   Kubernetes ◄┤
                         │                              │
                    AI-Unaware ←────────────────→ AI-Native
                         │                              │
              systemd ◄──┤                              │
              supervisord◄┤                              │
              K8s ◄──────┤                              │
                         │                   IAD ◄──────┤
                         │              LangGraph ◄─────┤
                         │               CrewAI ◄──────┤
```

**IAD's sweet spot**: Lightweight AND AI-Native — the only solution in that quadrant.

---

## 11. References

- [systemd Architecture](https://systemd.io/)
- [Kubernetes Concepts](https://kubernetes.io/docs/concepts/)
- [Actor Model (Erlang/OTP)](https://www.erlang.org/)
- [Tokio Runtime](https://tokio.rs/)
- [WASM Component Model](https://component-model.bytecodealliance.org/)
- [OpenTelemetry](https://opentelemetry.io/)
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
- [Capability-Based Security](https://en.wikipedia.org/wiki/Capability-based_security)

---

*© 2026 IKAMAI. All rights reserved.*
*Document ID: IAD-WP-001 | Reviewed quarterly.*
