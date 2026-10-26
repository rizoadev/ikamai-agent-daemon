# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Project scaffolding (Rust workspace)
- Complete documentation suite:
  - Whitepaper (vision, architecture, tech stack)
  - PRD (product requirements, acceptance criteria)
  - Technical Specification (data models, API spec, wire protocol)
  - Workflow Guide (branching, commits, CI/CD, releases)
  - Code of Conduct (Rust standards, review, security)
  - Master Plan (6 phases, 446h total estimate)
  - Roadmap (Oct 2026 → Jun 2027, 5 milestones)
  - Flowcharts (8 ASCII diagrams: startup, lifecycle, routing, etc.)
  - Testing Strategy (pyramid, 210+ unit tests planned, CI pipeline)
- GitHub repository (rizoadev/ikamai-agent-daemon)
- CI/CD pipeline (GitHub Actions)
- Linear project tracking

## [0.1.0-alpha.1] — TBD (Target: 2026-11-30)

### Planned
- Core daemon process (daemonize, PID file, signals)
- Configuration loading (TOML + env vars)
- SQLite storage layer
- Agent manifest parser (YAML)
- Agent registry (register/deregister/list)
- Process manager (spawn/kill/signal)
- Agent lifecycle (start → running → stop)
- Health monitor (heartbeat + auto-restart)
- CLI: `iad init/start/stop/status`
- CLI: `iad agent add/remove/list/start/stop/logs`
- Structured logging (tracing)

## [0.2.0-alpha.2] — TBD (Target: 2026-12-31)

### Planned
- Agent-daemon IPC (Unix sockets)
- Task routing with capability matching
- Task lifecycle + retry
- Event bus (pub/sub)
- Cron scheduler
- Python SDK (basic)

## [0.3.0-beta.1] — TBD (Target: 2027-02-28)

### Planned
- REST API (Axum)
- WebSocket real-time events
- API authentication
- Prometheus metrics
- Alert webhooks
- Hot-reload config
- Node.js SDK (basic)

## [0.5.0-beta.2] — TBD (Target: 2027-04-30)

### Planned
- gRPC inter-daemon communication
- Multi-node support
- WASM plugin runtime
- PostgreSQL backend
- mTLS + RBAC
- Secret management

## [1.0.0] — TBD (Target: 2027-06-30)

### Planned
- Production-ready release
- Complete documentation
- Performance benchmarks
- Security audit
- Docker + systemd deployment
- Example agents (5)
- Landing page

---

[Unreleased]: https://github.com/rizoadev/ikamai-agent-daemon/compare/HEAD
[0.1.0-alpha.1]: https://github.com/rizoadev/ikamai-agent-daemon/releases/tag/v0.1.0-alpha.1
