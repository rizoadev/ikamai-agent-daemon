# 🤖 IKAMAI Agent Daemon (IAD)

> A lightweight, AI-native daemon for orchestrating autonomous agent fleets.

[![CI](https://github.com/rizoadev/ikamai-agent-daemon/actions/workflows/ci.yml/badge.svg)](https://github.com/rizoadev/ikamai-agent-daemon/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

---

## What is IAD?

**IKAMAI Agent Daemon** is a persistent, event-driven orchestration daemon that manages, schedules, monitors, and coordinates a fleet of autonomous AI agents. Think of it as **systemd for AI agents** — but with built-in task routing, inter-agent messaging, and health monitoring.

```bash
# Initialize
iad init

# Register an agent
iad agent add agents/content-writer.yaml

# Start the daemon (all registered agents come alive)
iad start

# Submit a task (auto-routed to capable agent)
iad task submit '{"capabilities": ["text-generation"], "payload": {"prompt": "Write a blog post"}}'

# Check status
iad status
```

## ✨ Features

- 🔄 **Agent Lifecycle** — Start, stop, restart, auto-recover agents
- 🎯 **Task Routing** — Capability-based routing with priority queues
- 📡 **Event Bus** — Pub/sub + request/reply between agents
- ⏰ **Scheduler** — Cron-like scheduling with dependency chains
- 🛡️ **Policy Engine** — Rate limits, resource budgets, permission scopes
- 📊 **Observability** — Structured logs, Prometheus metrics, OpenTelemetry traces
- 🌐 **Multi-Node** — Scale from single machine to fleet with zero config change
- 🔌 **WASM Plugins** — Run untrusted agent code safely in sandbox
- 🐍 **Polyglot** — Python, Node.js, Go, Rust, or WASM agents

## 📐 Architecture

```
┌─────────────────────────────────────────────┐
│            IKAMAI Agent Daemon              │
│                                             │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────┐ │
│  │Registry│ │ Router │ │EventBus│ │Policy│ │
│  └───┬────┘ └───┬────┘ └───┬────┘ └──┬───┘ │
│      └──────────┴──────────┴─────────┘     │
│                Core Daemon                  │
│      ┌──────────┬──────────┬─────────┐     │
│  ┌───┴──┐ ┌─────┴──┐ ┌────┴───┐     │     │
│  │Agent1│ │Agent2  │ │Agent3  │ ... │     │
│  └──────┘ └────────┘ └────────┘     │     │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │   API (REST + gRPC + WebSocket)     │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
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
# Initialize daemon home directory
iad init

# Create a simple agent manifest
cat > ~/.iad/agents/hello.yaml << 'EOF'
name: "hello-agent"
version: "1.0.0"
runtime: "python3"
entrypoint: "hello.py"
capabilities:
  - "greeting"
health:
  check_interval: "30s"
  max_failures: 3
  restart_policy: "on-failure"
EOF

# Start the daemon
iad start

# Check status
iad status
iad agent list
```

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Whitepaper](docs/WHITEPAPER.md) | Vision, architecture, technology choices |
| [PRD](docs/PRD.md) | Product requirements & acceptance criteria |
| [Technical Spec](docs/SPEC.md) | Data models, API spec, wire protocol |
| [Workflow](docs/WORKFLOW.md) | Git flow, CI/CD, release process |
| [Code of Conduct](docs/CODE_OF_CONDUCT.md) | Coding standards, review guidelines |
| [Master Plan](docs/PLAN.md) | 6-phase development plan (446h) |
| [Roadmap](docs/ROADMAP.md) | Timeline, milestones, key dates |
| [Flowcharts](docs/FLOWCHART.md) | 8 ASCII architecture diagrams |
| [Testing](docs/TESTING.md) | Test strategy, 210+ tests planned |
| [Changelog](CHANGELOG.md) | Version history |

## 🗺️ Roadmap

| Milestone | Target | Status |
|-----------|--------|--------|
| v0.1.0-alpha.1 (Core Daemon) | Nov 2026 | 🔧 In Progress |
| v0.2.0-alpha.2 (Tasks & Events) | Dec 2026 | ⬜ Planned |
| v0.3.0-beta.1 (API & Monitoring) | Feb 2027 | ⬜ Planned |
| v0.5.0-beta.2 (Multi-Node) | Apr 2027 | ⬜ Planned |
| v1.0.0 (GA) | Jun 2027 | ⬜ Planned |

## 🛠️ Tech Stack

- **Language:** Rust (performance + safety)
- **Async:** Tokio
- **REST:** Axum
- **gRPC:** tonic
- **Storage:** SQLite (single-node) / PostgreSQL (multi-node)
- **Plugins:** WASM (wasmtime)
- **Observability:** OpenTelemetry
- **CLI:** clap

## 🤝 Contributing

We welcome contributions! Please read our [Code of Conduct](docs/CODE_OF_CONDUCT.md) and [Workflow Guide](docs/WORKFLOW.md) before submitting PRs.

```bash
# Development
cargo watch -x check -x test -x run

# Before committing
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

## 🏢 About IKAMAI

IKAMAI is an AI-first automation company building tools for the autonomous agent era. IAD is our core infrastructure project — the foundation everything else runs on.

- **Website:** [ikamai.id](https://ikamai.id) (coming soon)
- **GitHub:** [@rizoadev](https://github.com/rizoadev)

---

*Built with ❤️ and Rust by IKAMAI*
