# IKAMAI Agent Daemon — Whitepaper

**Version:** 0.1.0-draft  
**Date:** 2026-10-26  
**Author:** Rizoa (IKAMAI)  
**Status:** DRAFT

---

## Abstract

**IKAMAI Agent Daemon (IAD)** is a persistent, event-driven orchestration daemon that manages, schedules, monitors, and communicates across a fleet of autonomous AI agents. It acts as the central nervous system for IKAMAI's automation stack — coordinating agent lifecycle, inter-agent messaging, task routing, health monitoring, and failover — all from a single, always-on process.

## 1. Problem Statement

### 1.1 The Multi-Agent Chaos

Modern AI-powered businesses deploy multiple agents: coding agents, data scrapers, trading bots, content generators, customer support bots, DevOps monitors. Each runs in isolation with:

- **No unified lifecycle management** — agents crash silently, restart randomly.
- **No inter-agent communication** — agent A can't signal agent B.
- **No centralized scheduling** — cron jobs scattered across systems.
- **No health visibility** — no single pane of glass for all agents.
- **No policy enforcement** — agents run with unchecked permissions.

### 1.2 Existing Solutions Fall Short

| Solution | Gap |
|----------|-----|
| Kubernetes | Overkill for 5-50 agent processes; no AI-native primitives |
| Supervisord | No inter-agent messaging, no event routing |
| systemd | No multi-host, no task queue, no agent awareness |
| Custom scripts | Fragile, no observability, no recovery |

## 2. Vision

IAD provides a **lightweight, AI-native daemon** that:

1. **Manages agent lifecycle** — start, stop, restart, health-check, auto-recover
2. **Routes tasks** — assign work to agents based on capability, load, and priority
3. **Enables IPC** — pub/sub + RPC between agents (local and remote)
4. **Schedules jobs** — cron-like scheduling with dependency chains
5. **Enforces policies** — rate limits, resource budgets, permission scopes
6. **Observes everything** — structured logs, metrics, traces, alerts
7. **Scales horizontally** — single-node to multi-node fleet with zero config change

## 3. Architecture

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    IKAMAI Agent Daemon                   │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
│  │ Agent    │  │ Task     │  │ Event    │  │ Policy │  │
│  │ Registry │  │ Router   │  │ Bus      │  │ Engine │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └───┬────┘  │
│       │              │              │             │      │
│  ┌────┴──────────────┴──────────────┴─────────────┴──┐  │
│  │              Core Daemon Process                   │  │
│  │   (Event Loop + Scheduler + Health Monitor)        │  │
│  └────┬──────────────┬──────────────┬────────────────┘  │
│       │              │              │                    │
│  ┌────┴─────┐  ┌─────┴────┐  ┌─────┴─────┐             │
│  │ Agent A  │  │ Agent B  │  │ Agent C   │  ...         │
│  │ (worker) │  │ (worker) │  │ (worker)  │              │
│  └──────────┘  └──────────┘  └───────────┘              │
│                                                         │
│  ┌────────────────────────────────────────────────────┐  │
│  │           Storage Layer (SQLite/Postgres)          │  │
│  └────────────────────────────────────────────────────┘  │
│                                                         │
│  ┌────────────────────────────────────────────────────┐  │
│  │       API Layer (gRPC + REST + CLI + WebSocket)    │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Core Components

| Component | Responsibility |
|-----------|---------------|
| **Core Daemon** | Main event loop, signal handling, graceful shutdown |
| **Agent Registry** | Register/discover agents, capabilities, health status |
| **Task Router** | Route tasks to capable agents, load balancing |
| **Event Bus** | Pub/sub + request/reply messaging between agents |
| **Scheduler** | Cron-like job scheduling with dependency DAGs |
| **Policy Engine** | Rate limiting, resource budgets, permission scopes |
| **Health Monitor** | Heartbeat checks, auto-restart, circuit breakers |
| **Storage** | Persistent state, task queue, audit log |
| **API Layer** | External control plane (CLI, REST, gRPC, WebSocket) |

### 3.3 Agent Model

Each managed agent is defined by a **manifest**:

```yaml
# agent.manifest.yaml
name: "content-writer"
version: "1.2.0"
runtime: "python3.12"
entrypoint: "main.py"
capabilities:
  - "text-generation"
  - "seo-optimization"
resources:
  max_memory: "512Mi"
  max_cpu: "1.0"
  max_concurrent_tasks: 5
health:
  check_interval: "30s"
  timeout: "10s"
  max_failures: 3
  restart_policy: "on-failure"
dependencies:
  - "database-agent"
env:
  MODEL: "gpt-4o"
  TEMPERATURE: "0.7"
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
- **Remote IPC**: gRPC over mTLS (multi-node)
- **External**: REST/WebSocket API (human/tool integration)

## 4. Technology Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Language | **Rust** | Performance, safety, zero-cost abstractions |
| Async Runtime | Tokio | Battle-tested async I/O |
| IPC (local) | Unix Domain Sockets | Zero-copy, kernel-optimized |
| IPC (remote) | gRPC (tonic) | Multiplexed, typed, bidirectional |
| Storage | SQLite (embedded) + PostgreSQL (scale) | Simple start, scale later |
| Config | YAML/TOML + env vars | Human-friendly + 12-factor |
| API | Axum (REST) + tonic (gRPC) | Unified async stack |
| Observability | OpenTelemetry | Vendor-neutral traces/metrics/logs |
| CLI | clap | Ergonomic Rust CLI framework |
| Plugin System | WASM (wasmtime) | Sandboxed, portable agent plugins |

## 5. Key Differentiators

1. **AI-Native Primitives** — First-class concepts: agent, capability, task, tool — not just "processes"
2. **Zero-Config Single Node** — `iad init && iad start` — done
3. **WASM Plugin Agents** — Run untrusted agent code safely in WASM sandbox
4. **Capability-Based Routing** — Tasks find the right agent automatically
5. **Built-in Observability** — OpenTelemetry from day one, not bolted on
6. **Polyglot Agents** — Python, Node, Go, Rust, or WASM — daemon manages all
7. **Event Sourcing** — Full audit trail, replay, time-travel debugging

## 6. Use Cases

### 6.1 IKAMAI Internal
- Orchestrate coding agents (Pi/Claude Code) across repos
- Coordinate trading bots (XAUBot) with risk management
- Schedule content pipelines (Ahli News, Adobe Stock)
- Manage DevOps monitors across 8+ servers

### 6.2 External / Open Source
- Multi-agent AI application orchestration
- Microservice coordination for AI-heavy workloads
- Edge AI fleet management (IoT + AI agents)

## 7. Business Model

| Tier | Features | Price |
|------|----------|-------|
| **Community (OSS)** | Core daemon, 10 agents, local only | Free |
| **Pro** | Unlimited agents, multi-node, priority support | $29/mo |
| **Enterprise** | SSO, RBAC, audit logs, SLA, custom plugins | Custom |

## 8. Roadmap Summary

| Phase | Timeline | Deliverable |
|-------|----------|-------------|
| **v0.1 (Alpha)** | Month 1-2 | Core daemon, agent lifecycle, CLI |
| **v0.2 (Alpha)** | Month 2-3 | Task routing, event bus, scheduler |
| **v0.3 (Beta)** | Month 3-5 | REST API, health monitoring, storage |
| **v0.5 (Beta)** | Month 5-7 | Multi-node, gRPC, WASM plugins |
| **v1.0 (GA)** | Month 7-9 | Production-ready, docs, benchmarks |
| **v1.x** | Month 9+ | Dashboard UI, marketplace, enterprise |

## 9. Success Metrics

| Metric | Target (v1.0) |
|--------|---------------|
| Agent restart time | < 500ms |
| Task routing latency | < 10ms (local) |
| Event delivery latency | < 5ms (local) |
| Max concurrent agents | 100+ (single node) |
| Uptime | 99.9% |
| Memory overhead per agent | < 5MB |

## 10. References

- [systemd Architecture](https://systemd.io/)
- [Kubernetes Concepts](https://kubernetes.io/docs/concepts/)
- [Actor Model (Erlang/OTP)](https://www.erlang.org/)
- [Tokio Runtime](https://tokio.rs/)
- [WASM Component Model](https://component-model.bytecodealliance.org/)
- [OpenTelemetry](https://opentelemetry.io/)

---

*© 2026 IKAMAI. All rights reserved.*
