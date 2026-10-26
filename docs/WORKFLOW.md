# Workflow Guide

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-26

---

## 1. Development Workflow

### 1.1 Git Branching Strategy

```
main (stable, releases only)
  │
  ├── develop (integration branch)
  │     │
  │     ├── feature/FR1-agent-lifecycle
  │     ├── feature/FR2-task-router
  │     ├── feature/FR3-event-bus
  │     ├── fix/issue-42-restart-loop
  │     └── chore/update-deps
  │
  ├── release/v0.1.0
  ├── release/v0.2.0
  │
  └── hotfix/v0.1.1-crash-fix
```

### 1.2 Branch Naming Convention

```
feature/<module>-<short-desc>     # New features
fix/<issue-id>-<short-desc>       # Bug fixes
chore/<short-desc>                # Maintenance
docs/<short-desc>                 # Documentation
refactor/<module>-<short-desc>    # Refactoring
perf/<module>-<short-desc>        # Performance
test/<module>-<short-desc>        # Test additions
```

### 1.3 Commit Convention (Conventional Commits)

```
<type>(<scope>): <subject>

Types:
  feat     — New feature
  fix      — Bug fix
  docs     — Documentation
  style    — Formatting (no logic change)
  refactor — Code restructuring
  perf     — Performance improvement
  test     — Adding/fixing tests
  chore    — Build/tooling changes
  ci       — CI/CD changes

Scopes:
  daemon, registry, router, eventbus, scheduler, health,
  api, cli, storage, config, sdk, docs

Examples:
  feat(registry): add agent manifest validation
  fix(health): prevent restart loop on transient failure
  docs(api): add REST endpoint examples
  chore(ci): add release workflow
```

### 1.4 PR Workflow

```
1. Create branch from develop
2. Write code + tests
3. Run: cargo fmt && cargo clippy && cargo test
4. Push branch → Open PR to develop
5. CI runs: lint, test, build, security audit
6. Review: at least 1 approval
7. Squash merge to develop
8. For releases: develop → release/vX.Y.Z → main (merge)
```

## 2. Release Workflow

### 2.1 Release Process

```
1. Create release/vX.Y.Z from develop
2. Update version in Cargo.toml
3. Update CHANGELOG.md
4. Final testing on release branch
5. PR: release/vX.Y.Z → main
6. Tag: git tag -a vX.Y.Z -m "Release vX.Y.Z"
7. CI: build artifacts, publish crates, create GitHub release
8. Merge main back to develop
```

### 2.2 Versioning (SemVer)

```
MAJOR.MINOR.PATCH[-prerelease]

v0.1.0-alpha.1    Pre-release (breaking changes expected)
v0.5.0-beta.1     Beta (feature-complete, may have bugs)
v0.5.0-rc.1       Release candidate
v1.0.0            First stable release
v1.1.0            New features (backward compatible)
v1.1.1            Bug fixes only
v2.0.0            Breaking changes
```

## 3. CI/CD Pipeline

```yaml
# .github/workflows/ci.yml

on: [push, pull_request]

jobs:
  check:
    - cargo fmt --check
    - cargo clippy -- -D warnings
    - cargo audit

  test:
    matrix: [ubuntu-latest, macos-latest]
    - cargo test --all-features
    - cargo test --doc

  build:
    - cargo build --release
    - Upload artifacts

  coverage:
    - cargo tarpaulin --out Xml
    - Upload to codecov

  release: (on tag v*)
    - Build: linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64
    - Create GitHub Release
    - Publish to crates.io
    - Build & push Docker image
```

## 4. Development Environment

### 4.1 Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add clippy rustfmt

# Dev tools
cargo install cargo-watch cargo-tarpaulin cargo-audit cargo-fuzz

# Optional: protobuf (for gRPC)
sudo apt install protobuf-compiler  # Linux
brew install protobuf                # macOS
```

### 4.2 Development Commands

```bash
# Watch mode (auto-rebuild on change)
cargo watch -x check -x test -x run

# Run daemon in dev mode
cargo run -- start --foreground --log-level debug

# Run tests
cargo test                         # All tests
cargo test --lib                   # Unit tests only
cargo test --test integration      # Integration tests
cargo test -- --nocapture          # Show stdout

# Lint
cargo fmt --check
cargo clippy -- -D warnings

# Coverage
cargo tarpaulin --out Html

# Build release
cargo build --release

# Profile
cargo build --release && perf record ./target/release/iad start --foreground
```

## 5. Issue Tracking Workflow

### 5.1 Linear Integration

```
Label mapping:
  type:feature    → Feature request
  type:bug        → Bug report
  type:chore      → Maintenance task
  type:docs       → Documentation

Priority mapping:
  P0 → Urgent (1)
  P1 → High (2)
  P2 → Medium (3)
  P3 → Low (4)

State flow:
  Backlog → Todo → In Progress → In Review → Done
```

### 5.2 Issue Template

```markdown
## Description
[What needs to be done]

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2

## Technical Notes
[Implementation hints, references]

## Dependencies
[Blocked by / blocks]
```

## 6. Documentation Workflow

```
docs/
├── WHITEPAPER.md          # Vision and architecture (update quarterly)
├── PRD.md                 # Product requirements (update per phase)
├── SPEC.md                # Technical specification (update per feature)
├── WORKFLOW.md            # This file
├── CODE_OF_CONDUCT.md     # Community guidelines
├── PLAN.md                # Master plan and phases
├── ROADMAP.md             # Timeline and milestones
├── FLOWCHART.md           # Visual flows and diagrams
├── TESTING.md             # Test strategy and procedures
├── api/
│   ├── rest.md            # REST API docs
│   ├── grpc.md            # gRPC API docs
│   └── websocket.md       # WebSocket API docs
└── guides/
    ├── quickstart.md      # Getting started
    ├── agent-development.md # Writing agents
    └── deployment.md      # Production deployment
```

---

*Document maintained by Rizoa.*
