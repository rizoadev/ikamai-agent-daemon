# Product Requirements Document (PRD)

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-26  
**Author:** Rizoa  
**Status:** DRAFT

---

## 1. Overview

IAD is a system daemon that manages the lifecycle, scheduling, communication, and health of multiple AI agents. It provides a single control plane for all IKAMAI agent workloads.

## 2. Goals & Non-Goals

### Goals
- G1: Manage agent processes (start/stop/restart/health)
- G2: Route tasks to agents based on capabilities
- G3: Enable inter-agent communication (pub/sub + RPC)
- G4: Schedule recurring and one-shot jobs
- G5: Enforce resource limits and policies
- G6: Provide CLI, REST API, and WebSocket interfaces
- G7: Persist state across daemon restarts
- G8: Support horizontal scaling (multi-node)
- G9: Full observability (logs, metrics, traces)

### Non-Goals (v1.0)
- NG1: Web UI dashboard (deferred to v1.x)
- NG2: Agent marketplace / plugin store
- NG3: Built-in LLM inference (agents bring their own)
- NG4: Mobile app
- NG5: Windows support (Linux/macOS only for v1.0)

## 3. User Personas

### P1: Solo AI Developer
- Runs 3-5 agents on a single machine
- Wants simple setup, no Kubernetes
- Needs: `iad init`, `iad agent add`, `iad start`

### P2: DevOps/Platform Engineer
- Manages 20-50 agents across 3-5 servers
- Needs multi-node, monitoring, alerting
- Wants: Prometheus metrics, gRPC fleet management

### P3: AI Startup Team
- Building multi-agent AI product
- Needs: task routing, IPC, capability matching
- Wants: SDK for Python/Node, REST API

## 4. Functional Requirements

### FR1: Agent Lifecycle Management
| ID | Requirement | Priority |
|----|-------------|----------|
| FR1.1 | Register agent via manifest YAML | P0 |
| FR1.2 | Start agent process with resource limits | P0 |
| FR1.3 | Stop agent gracefully (SIGTERM → SIGKILL timeout) | P0 |
| FR1.4 | Restart agent on failure (configurable policy) | P0 |
| FR1.5 | List all agents with status | P0 |
| FR1.6 | View agent logs (tail/follow) | P0 |
| FR1.7 | Update agent manifest without restart | P1 |
| FR1.8 | Agent versioning and rollback | P2 |

### FR2: Task Router
| ID | Requirement | Priority |
|----|-------------|----------|
| FR2.1 | Submit task with required capabilities | P0 |
| FR2.2 | Route task to matching agent | P0 |
| FR2.3 | Task queue with priority levels (0-4) | P0 |
| FR2.4 | Task status tracking (pending/running/done/failed) | P0 |
| FR2.5 | Task retry with exponential backoff | P1 |
| FR2.6 | Task timeout and cancellation | P1 |
| FR2.7 | Task dependency chains (DAG) | P2 |
| FR2.8 | Load-based routing (least-busy agent) | P2 |

### FR3: Event Bus
| ID | Requirement | Priority |
|----|-------------|----------|
| FR3.1 | Publish event to topic | P0 |
| FR3.2 | Subscribe to topic(s) with filter | P0 |
| FR3.3 | Request/reply pattern | P1 |
| FR3.4 | Event persistence (replay from offset) | P1 |
| FR3.5 | Dead letter queue for failed deliveries | P2 |
| FR3.6 | Event schema validation | P2 |

### FR4: Scheduler
| ID | Requirement | Priority |
|----|-------------|----------|
| FR4.1 | Cron-expression scheduling | P0 |
| FR4.2 | One-shot delayed execution | P0 |
| FR4.3 | Schedule persistence across restarts | P0 |
| FR4.4 | Schedule enable/disable/delete | P1 |
| FR4.5 | Missed-fire policies (run-now / skip / queue) | P1 |
| FR4.6 | Dependency-based scheduling (run B after A) | P2 |

### FR5: Health Monitoring
| ID | Requirement | Priority |
|----|-------------|----------|
| FR5.1 | Periodic heartbeat check (configurable interval) | P0 |
| FR5.2 | Custom health check endpoints (HTTP/TCP/exec) | P1 |
| FR5.3 | Alert on agent failure (webhook/email) | P1 |
| FR5.4 | Circuit breaker (stop routing to unhealthy agent) | P2 |
| FR5.5 | Health history and uptime tracking | P2 |

### FR6: API & CLI
| ID | Requirement | Priority |
|----|-------------|----------|
| FR6.1 | CLI: `iad init/start/stop/status` | P0 |
| FR6.2 | CLI: `iad agent add/remove/list/logs` | P0 |
| FR6.3 | CLI: `iad task submit/status/cancel` | P0 |
| FR6.4 | REST API (JSON) for all operations | P0 |
| FR6.5 | WebSocket for real-time events | P1 |
| FR6.6 | gRPC API for inter-daemon communication | P1 |
| FR6.7 | API authentication (token-based) | P1 |

### FR7: Storage & Config
| ID | Requirement | Priority |
|----|-------------|----------|
| FR7.1 | SQLite storage for single-node | P0 |
| FR7.2 | YAML/TOML daemon configuration | P0 |
| FR7.3 | Environment variable overrides | P0 |
| FR7.4 | PostgreSQL backend for multi-node | P2 |
| FR7.5 | Encrypted secrets storage | P2 |

### FR8: Observability
| ID | Requirement | Priority |
|----|-------------|----------|
| FR8.1 | Structured JSON logging | P0 |
| FR8.2 | Prometheus-compatible metrics endpoint | P1 |
| FR8.3 | OpenTelemetry trace export | P2 |
| FR8.4 | Audit log for all mutations | P2 |

## 5. Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR1 | Agent start latency | < 200ms (excl. agent boot) |
| NFR2 | Task routing latency | < 10ms |
| NFR3 | Event delivery latency | < 5ms (local) |
| NFR4 | Max agents (single node) | 100+ |
| NFR5 | Daemon memory footprint | < 50MB base |
| NFR6 | Daemon startup time | < 2s |
| NFR7 | Config reload | Hot reload, no downtime |
| NFR8 | Crash recovery | Resume from persisted state |
| NFR9 | Platform | Linux (primary), macOS (secondary) |

## 6. Security Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| SEC1 | API authentication (bearer token / mTLS) | P1 |
| SEC2 | Agent isolation (process-level, cgroups optional) | P1 |
| SEC3 | Secret injection (not in manifest plaintext) | P1 |
| SEC4 | TLS for all remote communication | P1 |
| SEC5 | RBAC for API operations | P2 |
| SEC6 | Audit logging for compliance | P2 |

## 7. Acceptance Criteria

### MVP (v0.1)
- [ ] Daemon starts and stays running as background process
- [ ] Register agent via YAML manifest
- [ ] Start/stop/restart agent process
- [ ] Health check with auto-restart
- [ ] CLI for basic operations
- [ ] SQLite persistent state
- [ ] Structured logging

### Beta (v0.5)
- [ ] Task routing with capability matching
- [ ] Event bus (pub/sub)
- [ ] Cron scheduler
- [ ] REST API
- [ ] Prometheus metrics
- [ ] Multi-agent integration test

### GA (v1.0)
- [ ] All P0 + P1 requirements met
- [ ] Multi-node support
- [ ] gRPC inter-daemon
- [ ] Benchmark: 100 agents, < 50MB daemon overhead
- [ ] Documentation complete
- [ ] Security audit passed

## 8. Dependencies

| Dependency | Type | Notes |
|-----------|------|-------|
| Rust 1.80+ | Build | MSRV |
| Tokio 1.x | Runtime | Async I/O |
| SQLite 3.x | Storage | Embedded |
| OpenSSL / rustls | Security | TLS |
| protobuf | Build | gRPC definitions |

## 9. Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Rust learning curve | Slower dev | Medium | Start with core, iterate |
| Scope creep | Delayed release | High | Strict phase gating |
| Agent compatibility | Limited adoption | Medium | Polyglot agent support |
| Security vulnerabilities | Trust loss | Low | Security audit at beta |

---

*Document maintained by Rizoa. Review quarterly.*
