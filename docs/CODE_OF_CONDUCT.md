# Code of Conduct

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 1.0  
**Date:** 2026-10-26

---

## 1. Code Standards

### 1.1 Rust Style Guide

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- All code must pass `cargo fmt` (default rustfmt config)
- All code must pass `cargo clippy -- -D warnings` with zero warnings
- Use `#[must_use]` for functions with important return values
- Prefer `thiserror` for library errors, `anyhow` for application errors
- No `unsafe` without explicit comment justifying safety

### 1.2 Naming Conventions

```rust
// Modules: snake_case
mod agent_registry;
mod task_router;

// Types: PascalCase
struct AgentManifest { ... }
enum AgentStatus { Running, Stopped, Failed }

// Functions/methods: snake_case
fn register_agent(manifest: &AgentManifest) -> Result<AgentId> { ... }

// Constants: SCREAMING_SNAKE_CASE
const MAX_AGENT_FAILURES: u32 = 3;
const DEFAULT_HEALTH_INTERVAL: Duration = Duration::from_secs(30);

// Traits: PascalCase, often adjective/verb
trait Routable { ... }
trait HealthCheckable { ... }
```

### 1.3 File Organization

```rust
// Per file, in order:
// 1. Module docs (//!)
// 2. Imports (std, external crates, internal modules)
// 3. Constants
// 4. Type definitions (structs, enums)
// 5. Trait definitions
// 6. Implementations (impl, trait impl)
// 7. Functions
// 8. Tests (#[cfg(test)] mod tests)
```

### 1.4 Error Handling

```rust
// Define domain errors with thiserror
#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("Agent '{name}' not found")]
    AgentNotFound { name: String },
    
    #[error("Agent '{name}' is already running (pid={pid})")]
    AgentAlreadyRunning { name: String, pid: u32 },
    
    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Use Result type alias
pub type Result<T> = std::result::Result<T, DaemonError>;
```

### 1.5 Documentation

```rust
/// Register a new agent with the daemon.
///
/// The agent manifest is validated before registration. If an agent
/// with the same name already exists, returns `AgentAlreadyExists`.
///
/// # Arguments
/// * `manifest` - The agent manifest (parsed from YAML)
///
/// # Returns
/// The assigned agent ID on success.
///
/// # Errors
/// * `AgentAlreadyExists` - Agent name is taken
/// * `InvalidManifest` - Manifest validation failed
///
/// # Examples
/// ```
/// let id = registry.register(manifest)?;
/// println!("Registered agent: {id}");
/// ```
pub fn register(&self, manifest: AgentManifest) -> Result<AgentId> { ... }
```

## 2. Git & PR Standards

### 2.1 Commit Rules

1. **One logical change per commit** — don't mix feature + refactor
2. **Conventional Commits** format (see WORKFLOW.md)
3. **Sign commits** with GPG (`git commit -S`)
4. **No merge commits** in feature branches (rebase instead)
5. **No WIP commits** in PRs (squash before review)

### 2.2 PR Requirements

- [ ] Descriptive title following conventional commits
- [ ] Description with context and motivation
- [ ] All CI checks pass (fmt, clippy, test, audit)
- [ ] New features have tests
- [ ] New APIs have documentation
- [ ] Breaking changes noted in description
- [ ] No TODO/FIXME left without linked issue

### 2.3 Review Guidelines

**For reviewers:**
- Focus on correctness, clarity, and maintainability
- Check error handling paths
- Verify test coverage for edge cases
- Be constructive — suggest alternatives, not just "wrong"
- Approve when ready, don't nitpick style (that's fmt/clippy's job)

**For authors:**
- Keep PRs small (< 400 lines preferred, < 800 max)
- Respond to all review comments
- Don't force-push after review starts (use fixup commits)

## 3. Security Standards

### 3.1 Rules

1. **No secrets in code** — use env vars or secret store
2. **No secrets in logs** — sanitize before logging
3. **No `unsafe`** without safety proof in comments
4. **Pin dependencies** — use exact versions in Cargo.lock
5. **Audit regularly** — `cargo audit` in CI
6. **Minimize dependencies** — fewer deps = fewer CVEs
7. **Validate all input** — never trust user/agent input

### 3.2 Dependency Policy

```toml
# Cargo.toml — pin major versions minimum
tokio = "1"           # Stable async runtime
serde = "1"           # Stable serialization
axum = "0.7"          # Pin minor for pre-1.0
```

- Run `cargo audit` in CI — fail build on known vulnerabilities
- Review new dependencies before adding (check maintainer, downloads, audit history)
- Prefer Rust ecosystem crates with `#![forbid(unsafe_code)]`

## 4. Testing Standards

### 4.1 Test Requirements

| Change Type | Test Required |
|-------------|--------------|
| New feature | Unit + integration test |
| Bug fix | Regression test (reproduces bug) |
| Refactor | Existing tests must still pass |
| API change | API test update |
| Config change | Config parsing test |

### 4.2 Test Organization

```rust
// Unit tests: inline in source file
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn register_agent_success() { ... }
    
    #[test]
    fn register_agent_duplicate_name_fails() { ... }
}

// Integration tests: tests/ directory
// tests/agent_lifecycle.rs
// tests/task_routing.rs
// tests/event_bus.rs
```

### 4.3 Test Naming

```
<action>_<condition>_<expected_result>

register_agent_with_valid_manifest_succeeds
register_agent_with_duplicate_name_returns_error
route_task_with_no_matching_agent_queues_task
health_check_after_max_failures_marks_agent_failed
```

## 5. Community Standards

### 5.1 Contributor Covenant

This project follows the [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).

**Our Pledge:** We pledge to make participation in our community a harassment-free experience for everyone.

**Our Standards:**
- ✅ Using welcoming and inclusive language
- ✅ Being respectful of differing viewpoints
- ✅ Gracefully accepting constructive criticism
- ✅ Focusing on what is best for the community
- ❌ Trolling, insulting/derogatory comments
- ❌ Public or private harassment
- ❌ Publishing others' private information

### 5.2 Issue & Discussion Etiquette

1. Search before opening a new issue
2. Use issue templates
3. Provide reproduction steps for bugs
4. Be patient — maintainers are humans too
5. Celebrate contributions — every PR matters

---

*Maintained by Rizoa. Updated as standards evolve.*
