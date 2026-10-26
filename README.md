# 🤖 IKAMAI Agent Daemon (IAD)

> **systemd for AI agents** — A lightweight, AI-native daemon for orchestrating autonomous agent fleets.

[![CI](https://github.com/rizoadev/ikamai-agent-daemon/actions/workflows/ci.yml/badge.svg)](https://github.com/rizoadev/ikamai-agent-daemon/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.1.0--alpha-blue.svg)](https://github.com/rizoadev/ikamai-agent-daemon/releases)

---

## What is IAD?

**IKAMAI Agent Daemon** is a persistent, event-driven orchestration daemon that manages, schedules, monitors, and coordinates a fleet of autonomous AI agents. Think of it as **systemd for AI agents** — but with built-in task routing, inter-agent messaging, capability-based scheduling, and full observability.

```bash
# Initialize
iad init

# Register an agent
iad agent add agents/content-writer.yaml

# Start the daemon (all registered agents come alive)
iad start

# Submit a task (auto-routed to capable agent)
iad task submit '{"capabilities": ["text-generation"], "payload": {"prompt": "Write a blog post"}}'

# Check fleet status
iad status

# View agent logs
iad agent logs content-writer
```

## ✨ Features

| Feature | Description | Status |
|---------|-------------|--------|
| 🔄 **Agent Lifecycle** | Start, stop, restart, auto-recover agents | Phase 1 |
| 🎯 **Task Routing** | Capability-based routing with priority queues | Phase 2 |
| 📡 **Event Bus** | Pub/sub + request/reply between agents | Phase 2 |
| ⏰ **Scheduler** | Cron-like scheduling with dependency chains | Phase 2 |
| 🌐 **REST API** | Full control plane via HTTP | Phase 3 |
| 📊 **Observability** | Structured logs, Prometheus metrics, OpenTelemetry | Phase 3 |
| 🛡️ **Policy Engine** | Rate limits, resource budgets, permission scopes | Phase 4 |
| 🌍 **Multi-Node** | Scale from single machine to fleet | Phase 4 |
| 🔌 **WASM Plugins** | Run untrusted agent code safely in sandbox | Phase 4 |
| 🐍 **Polyglot** | Python, Node.js, Go, Rust, or WASM agents | Phase 2+ |

## 📐 Architecture

```
┌─────────────────────────────────────────────────┐
│              IKAMAI Agent Daemon                 │
│                                                  │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌───────┐  │
│  │Registry│  │ Router │  │EventBus│  │Policy │  │
│  └───┬────┘  └───┬────┘  └───┬────┘  └──┬────┘  │
│      └───────────┴──────────┴────────────┘       │
│                   Core Daemon                     │
│      ┌───────────┬──────────┬────────────┐       │
│  ┌───┴───┐ ┌─────┴──┐ ┌────┴───┐        │       │
│  │Agent1 │ │Agent2  │ │Agent3  │  ...    │       │
│  │Python │ │Node.js │ │WASM    │         │       │
│  └───────┘ └────────┘ └────────┘         │       │
│                                                  │
│  ┌─────────────────────────────────────────────┐ │
│  │   API (CLI + REST + gRPC + WebSocket)       │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.80+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

### Build & Install

```bash
git clone https://github.com/rizoadev/ikamai-agent-daemon.git
cd ikamai-agent-daemon
cargo build --release
cp target/release/iad ~/.local/bin/
```

### First Run

```bash
# 1. Initialize
iad init

# 2. Start daemon
iad start

# 3. Register your first agent
cat > my-agent.yaml << 'EOF'
name: "hello-world"
version: "1.0.0"
runtime: "bash"
entrypoint: "echo 'Hello from IAD!'"
capabilities:
  - "greeting"
health:
  check_interval: "30s"
  max_failures: 3
  restart_policy: "on-failure"
EOF

iad agent add my-agent.yaml

# 4. Check status
iad agent list

# 5. Stop daemon
iad stop
```

### Docker

```bash
docker run -d --name iad \
  -v iad-data:/root/.iad \
  -p 7700:7700 \
  ghcr.io/rizoadev/iad:latest
```

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Whitepaper](docs/WHITEPAPER.md) | Vision, architecture, competitive landscape |
| [PRD](docs/PRD.md) | Product requirements & acceptance criteria |
| [Technical Spec](docs/SPEC.md) | Data models, API spec, wire protocol |
| [Architecture](docs/ARCHITECTURE.md) | System design, crate structure, ADRs |
| [Master Plan](docs/PLAN.md) | 6-phase development plan with estimates |
| [Roadmap](docs/ROADMAP.md) | Visual timeline & milestone dates |
| [Timeline](docs/TIMELINE.md) | Week-by-week schedule & sprint planning |
| [Flowcharts](docs/FLOWCHART.md) | Visual flows (startup, lifecycle, routing) |
| [Workflow](docs/WORKFLOW.md) | Git branching, commits, CI/CD, releases |
| [Versioning](docs/VERSIONING.md) | SemVer strategy, compatibility matrix |
| [Code of Conduct](docs/CODE_OF_CONDUCT.md) | Code standards, review, security |
| [Testing](docs/TESTING.md) | Test pyramid, strategy, CI pipeline |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Changelog](CHANGELOG.md) | Version history |

## 🗂️ Project Structure

```
ikamai-agent-daemon/
├── crates/
│   ├── iad/           # CLI binary (clap)
│   ├── iad-core/      # Domain logic (models, registry, router, eventbus)
│   ├── iad-storage/   # Persistence (SQLite, PostgreSQL)
│   ├── iad-api/       # API servers (REST, gRPC, WebSocket)
│   └── iad-sdk/       # Rust SDK for building agents
├── sdks/              # Python & Node.js SDKs
├── examples/          # Example agents
├── tests/             # Integration & E2E tests
├── docs/              # Documentation suite (13 docs)
├── .github/           # CI/CD workflows
├── Cargo.toml         # Workspace manifest
├── Dockerfile
├── LICENSE            # MIT
└── README.md          # This file
```

## 🗺️ Roadmap

| Phase | Version | Target | Status |
|-------|---------|--------|--------|
| Foundation | v0.0.1 | Oct 2026 | ✅ Done |
| Core Daemon | v0.1.0-alpha.1 | Nov 2026 | 🔧 Active |
| Tasks & Events | v0.2.0-alpha.2 | Dec 2026 | ⬜ Planned |
| API & Monitoring | v0.3.0-beta.1 | Feb 2027 | ⬜ Planned |
| Scale & Plugins | v0.5.0-beta.2 | Apr 2027 | ⬜ Planned |
| Production GA | v1.0.0 | Jun 2027 | ⬜ Planned |

## 📊 Metrics Targets (v1.0)

| Metric | Target |
|--------|--------|
| Agent restart | < 500ms |
| Task routing | < 10ms |
| Event delivery | < 5ms |
| Max agents (single node) | 100+ |
| Daemon memory | < 50MB |

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

## 🔗 Links

- **GitHub:** [rizoadev/ikamai-agent-daemon](https://github.com/rizoadev/ikamai-agent-daemon)
- **Linear:** [IKAMAI Agent Daemon Project](https://linear.app/ikamai/project/ikamai-agent-daemon-iad-bec46a51)
- **Author:** [Rizoa](https://github.com/rizoadev) (IKAMAI)

---

*Built with ❤️ and 🦀 Rust by IKAMAI*
