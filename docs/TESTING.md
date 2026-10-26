# Testing Strategy & Plan

**Product:** IKAMAI Agent Daemon (IAD)  
**Date:** 2026-10-26  
**Author:** Rizoa

---

## 1. Testing Pyramid

```
         ▲
        / \
       / E2E \          5-10 tests    (full scenarios)
      /───────\
     / Integr. \        20-40 tests   (cross-module)
    /───────────\
   /    Unit     \      200+ tests    (per function)
  /───────────────\
 /    Static Anal. \    CI (fmt, clippy, audit)
/═══════════════════\
```

## 2. Test Categories

### 2.1 Static Analysis (CI — Every Push)

| Tool | Purpose | Command |
|------|---------|---------|
| `cargo fmt` | Code formatting | `cargo fmt --check` |
| `cargo clippy` | Lint + common mistakes | `cargo clippy -- -D warnings` |
| `cargo audit` | Known CVE check | `cargo audit` |
| `cargo deny` | License + advisory check | `cargo deny check` |

### 2.2 Unit Tests (Per Module)

**Coverage Target:** 80%+

| Module | Test Count (est.) | Key Tests |
|--------|------------------|-----------|
| Config parser | 15 | Valid TOML, missing fields, env override, defaults |
| Agent manifest | 15 | Valid YAML, missing fields, invalid values, version |
| Agent registry | 20 | Register, duplicate name, deregister, list, filter |
| Process manager | 15 | Spawn, kill, signal, PID tracking, exit codes |
| Task model | 10 | Create, validate, serialize, priority ordering |
| Task router | 20 | Match caps, no match, load balance, priority sort |
| Event bus | 15 | Publish, subscribe, filter, unsubscribe |
| Scheduler | 15 | Cron parse, next_run calc, one-shot, enable/disable |
| Storage (SQLite) | 20 | CRUD all tables, migrations, concurrent access |
| Health monitor | 15 | Check pass, fail, max failures, backoff calc |
| IPC protocol | 15 | Frame encode/decode, large payload, malformed |
| CLI parser | 15 | All subcommands, missing args, help text |
| API handlers | 20 | Request/response for each endpoint |

**Total estimated:** ~210 unit tests

### 2.3 Integration Tests (Cross-Module)

| Test Suite | Description | Key Scenarios |
|-----------|-------------|---------------|
| `agent_lifecycle` | Full agent start→run→stop | Start agent, verify PID, stop gracefully |
| `agent_recovery` | Crash + auto-restart | Kill agent, verify restart, max failures |
| `task_routing` | End-to-end task flow | Submit task, route, execute, return result |
| `event_flow` | Pub/sub across agents | Agent A publishes, Agent B receives |
| `scheduler` | Scheduled task execution | Create schedule, wait, verify task submitted |
| `api_crud` | REST API operations | Create/read/update/delete via HTTP |
| `api_auth` | Authentication | Valid token, invalid token, no token |
| `websocket` | Real-time events | Connect, subscribe, receive events |
| `persistence` | State survival | Start daemon, register agents, restart daemon, verify state |
| `concurrent` | Race conditions | Parallel task submit, parallel agent register |

**Total estimated:** ~30 integration tests

### 2.4 End-to-End Tests (Full Scenarios)

| Test | Description | Steps |
|------|-------------|-------|
| E2E-1 | First-time setup | `iad init` → `iad start` → `iad status` |
| E2E-2 | Agent lifecycle | Add agent → start → health check → stop → remove |
| E2E-3 | Task pipeline | Submit task → route → execute → get result |
| E2E-4 | Multi-agent | 3 agents, different caps → route tasks correctly |
| E2E-5 | Crash recovery | Kill daemon → restart → agents resume |
| E2E-6 | Scheduled job | Create cron schedule → wait → verify execution |
| E2E-7 | API-driven | All operations via REST API (curl/httpie) |
| E2E-8 | SDK integration | Python agent with SDK → register → handle task |

**Total estimated:** ~8 E2E tests

### 2.5 Performance Tests

| Test | Tool | Target |
|------|------|--------|
| Task throughput | k6 / custom | 1000 tasks/s submitted |
| Event throughput | custom | 5000 events/s published |
| API latency | wrk | p99 < 20ms for REST |
| Agent start time | custom | < 200ms |
| Memory under load | valgrind/heaptrack | < 50MB base + < 5MB/agent |
| 100 agents soak | custom | 1h stable, no leak, no crash |

### 2.6 Fuzz Tests

| Target | Tool | Purpose |
|--------|------|---------|
| IPC wire protocol | `cargo-fuzz` | Malformed frames, huge payloads |
| Config parser | `cargo-fuzz` | Invalid TOML |
| Manifest parser | `cargo-fuzz` | Invalid YAML |
| REST API inputs | `cargo-fuzz` | Malformed JSON bodies |
| Cron expression | `cargo-fuzz` | Edge case cron strings |

### 2.7 Security Tests

| Test | Method | What |
|------|--------|------|
| Dependency audit | `cargo audit` | Known CVEs in deps |
| API auth bypass | Manual | Unauthenticated access attempts |
| IPC injection | Fuzz | Malicious IPC messages |
| Path traversal | Manual | Agent manifest path exploitation |
| Secret leakage | grep + manual | Secrets in logs/output |
| Resource exhaustion | Load test | OOM, FD exhaustion, CPU spin |

## 3. Test Infrastructure

### 3.1 CI Pipeline (GitHub Actions)

```yaml
jobs:
  static:
    steps:
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - cargo audit
  
  unit:
    matrix: [ubuntu-latest, macos-latest]
    steps:
      - cargo test --lib --all-features
      - cargo test --doc
  
  integration:
    steps:
      - cargo test --test '*' --all-features
  
  coverage:
    steps:
      - cargo tarpaulin --out Xml
      - Upload codecov
  
  e2e:
    steps:
      - cargo build --release
      - python3 tests/e2e/run_all.py
```

### 3.2 Test Helpers

```rust
// tests/common/mod.rs

/// Spawn a test daemon with a temporary directory
pub struct TestDaemon {
    pub home: TempDir,
    pub process: Child,
    pub api_port: u16,
}

impl TestDaemon {
    pub async fn start() -> Self { ... }
    pub async fn stop(&mut self) { ... }
    pub fn cli(&self, args: &[&str]) -> Command { ... }
    pub fn api_url(&self, path: &str) -> String { ... }
}

/// Create a minimal test agent that echoes tasks
pub fn create_echo_agent(dir: &Path) -> PathBuf { ... }

/// Create a test agent that crashes after N tasks
pub fn create_crash_agent(dir: &Path, crash_after: u32) -> PathBuf { ... }
```

## 4. Test Execution Schedule

| Test Type | Trigger | Duration |
|-----------|---------|----------|
| Static analysis | Every push | ~1 min |
| Unit tests | Every push | ~2 min |
| Integration tests | Every PR | ~5 min |
| E2E tests | PR to develop/main | ~10 min |
| Performance tests | Weekly / pre-release | ~30 min |
| Fuzz tests | Weekly (background) | ~1 hour |
| Security audit | Monthly / pre-release | ~2 hours |

## 5. Test Reporting

| Metric | Tool | Target |
|--------|------|--------|
| Code coverage | tarpaulin → Codecov | > 80% |
| Test pass rate | GitHub Actions | 100% (block merge) |
| Performance regression | Custom benchmark CI | No regression > 10% |
| Security issues | cargo-audit | 0 known CVEs |

## 6. Bug Classification

| Severity | Definition | SLA |
|----------|-----------|-----|
| S1 - Critical | Daemon crash, data loss, security breach | Fix in 24h |
| S2 - Major | Feature broken, workaround exists | Fix in 3 days |
| S3 - Minor | UI/UX issue, non-critical bug | Fix in 1 week |
| S4 - Trivial | Typo, cosmetic, nice-to-have | Next release |

---

*Testing plan maintained by Rizoa.*
