# Contributing Guide

**Product:** IKAMAI Agent Daemon (IAD)  
**Date:** 2026-10-26  
**Author:** Rizoa

---

Thank you for your interest in contributing to IKAMAI Agent Daemon! 🤖

## Quick Start

```bash
# Fork & clone
git clone https://github.com/YOUR_USERNAME/ikamai-agent-daemon.git
cd ikamai-agent-daemon

# Setup Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add clippy rustfmt

# Build
cargo build

# Run tests
cargo test

# Check formatting & lints
cargo fmt --check
cargo clippy -- -D warnings
```

---

## 1. Ways to Contribute

| Type | Difficulty | How |
|------|-----------|-----|
| 🐛 Bug report | Easy | Open an issue with reproduction steps |
| 📝 Documentation | Easy | Fix typos, improve examples, add guides |
| ✅ Tests | Medium | Add unit/integration/E2E tests |
| 🔧 Bug fix | Medium | Fix an existing issue, submit PR |
| ✨ Feature | Hard | Discuss in issue first, then implement |
| 🌍 Translation | Easy | Translate docs to other languages |
| 🧪 Testing | Medium | Test pre-releases, report findings |
| 📢 Advocacy | Easy | Blog posts, talks, social media |

---

## 2. Development Setup

### 2.1 Prerequisites

```bash
# Required
rustup (with Rust 1.80+)
git

# Recommended
cargo-watch     # Auto-rebuild on change
cargo-tarpaulin # Code coverage
cargo-audit     # Security audit
cargo-fuzz      # Fuzz testing

# Install recommended tools
cargo install cargo-watch cargo-tarpaulin cargo-audit

# Optional: protobuf (for gRPC, Phase 4+)
sudo apt install protobuf-compiler   # Linux
brew install protobuf                 # macOS
```

### 2.2 Development Commands

```bash
# Watch mode (auto-rebuild + test on change)
cargo watch -x check -x test -x clippy

# Run daemon in dev mode
RUST_LOG=debug cargo run -- start --foreground

# Run specific test
cargo test test_name -- --nocapture

# Coverage report
cargo tarpaulin --out Html && open tarpaulin-report.html

# Profile build
cargo build --release --timings

# Benchmark
cargo bench
```

---

## 3. Workflow

### 3.1 For Bug Fixes

```
1. Find or create an issue
2. Comment "I'll work on this" 
3. Fork → create branch: fix/<issue-id>-<short-desc>
4. Write regression test that fails
5. Fix the bug
6. Verify test passes
7. cargo fmt && cargo clippy && cargo test
8. Submit PR referencing the issue
```

### 3.2 For New Features

```
1. Open a feature request issue FIRST
2. Discuss design with maintainers
3. Wait for approval (label: "approved")
4. Fork → create branch: feature/<module>-<short-desc>
5. Implement with tests
6. Update docs if needed
7. cargo fmt && cargo clippy && cargo test
8. Submit PR referencing the issue
```

### 3.3 Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

Types: feat, fix, docs, style, refactor, perf, test, chore, ci
Scopes: daemon, registry, router, eventbus, scheduler, health,
        api, cli, storage, config, sdk, docs

Examples:
  feat(registry): add agent manifest validation
  fix(health): prevent restart loop on transient failure
  docs(api): add REST endpoint examples
  test(router): add capability matching edge cases
```

### 3.4 PR Template

```markdown
## Summary
[What does this PR do?]

## Related Issue
Closes #<issue-number>

## Changes
- [Change 1]
- [Change 2]

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests (if applicable)
- [ ] Manual testing performed

## Checklist
- [ ] cargo fmt --check passes
- [ ] cargo clippy -- -D warnings passes
- [ ] cargo test passes
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG.md updated (if user-facing change)
- [ ] No TODO/FIXME without linked issue
```

---

## 4. Code Standards

### 4.1 Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- All code must pass `cargo fmt` and `cargo clippy -- -D warnings`
- Prefer `thiserror` for library errors, `anyhow` for application errors
- No `unsafe` without explicit safety comment
- Use `#[must_use]` for functions with important return values
- Write doc comments for all public items

### 4.2 Testing Standards

| Change Type | Test Required |
|-------------|--------------|
| New feature | Unit + integration test |
| Bug fix | Regression test (reproduces the bug) |
| Refactor | Existing tests must pass |
| API change | API test update |

### 4.3 Documentation Standards

- All public functions, structs, enums, traits have doc comments
- Include `# Examples` in doc comments when helpful
- Update relevant docs/ files for user-facing changes

---

## 5. Review Process

1. **CI must pass** — all checks green
2. **At least 1 approval** from maintainer
3. **Squash merge** to develop branch
4. Reviewer focuses on: correctness, clarity, test coverage
5. Keep PRs small (<400 lines preferred, <800 max)

---

## 6. Getting Help

- **Issues**: [GitHub Issues](https://github.com/rizoadev/ikamai-agent-daemon/issues)
- **Discussions**: [GitHub Discussions](https://github.com/rizoadev/ikamai-agent-daemon/discussions)
- **Email**: rizoa@ikamai.id

---

*We value every contribution, no matter how small. Thank you!*
