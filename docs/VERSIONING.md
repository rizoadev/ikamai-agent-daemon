# Versioning Strategy

**Product:** IKAMAI Agent Daemon (IAD)  
**Date:** 2026-10-26  
**Author:** Rizoa  
**Document ID:** IAD-VER-001

---

## 1. Semantic Versioning (SemVer 2.0.0)

IAD follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html):

```
MAJOR.MINOR.PATCH[-prerelease][+build]

Examples:
  0.1.0-alpha.1       First pre-release (breaking changes expected)
  0.5.0-beta.1        Feature-complete beta (may have bugs)
  0.5.0-rc.1          Release candidate (production-intent)
  1.0.0               First stable General Availability release
  1.1.0               Backward-compatible new features
  1.1.1               Backward-compatible bug fixes
  2.0.0               Breaking API/config changes
```

### 1.1 Version Bump Rules

| Change Type | Version Bump | Example |
|------------|-------------|---------|
| Breaking API change | MAJOR | CLI flag renamed, REST endpoint removed, config key changed |
| New feature (backward-compatible) | MINOR | New CLI command, new API endpoint, new config option |
| Bug fix (backward-compatible) | PATCH | Crash fix, performance improvement, documentation |
| Pre-release | `-alpha.N` / `-beta.N` / `-rc.N` | Testing builds |

### 1.2 Pre-Release Stages

```
alpha  → Active development, unstable API, breaking changes expected
         Target audience: core developers
         
beta   → Feature-complete, API mostly stable, may have bugs
         Target audience: early adopters, testers
         
rc     → Release candidate, production-intent, last chance for fixes
         Target audience: brave production users
         
(none) → General Availability, stable, production-ready
         Target audience: everyone
```

---

## 2. Release Schedule

### 2.1 Version → Phase Mapping

| Version | Phase | Codename | Target Date | Status |
|---------|-------|----------|-------------|--------|
| `0.0.1` | Phase 0 — Foundation | `scaffold` | 2026-10-31 | ✅ DONE |
| `0.1.0-alpha.1` | Phase 1 — Core Daemon | `heartbeat` | 2026-11-30 | 🔧 ACTIVE |
| `0.2.0-alpha.2` | Phase 2 — Tasks & Events | `synapse` | 2026-12-31 | ⬜ PLANNED |
| `0.3.0-beta.1` | Phase 3 — API & Monitoring | `lighthouse` | 2027-02-28 | ⬜ PLANNED |
| `0.5.0-beta.2` | Phase 4 — Scale & Plugins | `constellation` | 2027-04-30 | ⬜ PLANNED |
| `0.9.0-rc.1` | Pre-production | `launchpad` | 2027-06-15 | ⬜ PLANNED |
| `1.0.0` | Phase 5 — GA | `genesis` | 2027-06-30 | ⬜ PLANNED |
| `1.1.0` | Phase 6 — Dashboard | `horizon` | 2027-09-30 | ⬜ PLANNED |
| `2.0.0` | Next Gen | `nova` | 2028-Q2 | ⬜ PLANNED |

### 2.2 Release Cadence

| Stage | Cadence | Notes |
|-------|---------|-------|
| Pre-v1.0 (alpha/beta) | Every 4-8 weeks | Fast iteration, breaking changes OK |
| v1.x (stable) | Every 4-6 weeks (MINOR), as needed (PATCH) | Backward-compatible only |
| v2.0+ (major) | Every 12-18 months | Breaking changes, migration guide required |
| Security patches | As soon as possible | Hotfix → patch release within 48 hours |

---

## 3. Cargo Workspace Versioning

All crates in the workspace share the **same version**:

```toml
# Cargo.toml (workspace root)
[workspace.package]
version = "0.1.0"

# Each crate inherits:
# crates/iad/Cargo.toml
[package]
version.workspace = true

# crates/iad-core/Cargo.toml
[package]
version.workspace = true
```

### 3.1 Version Bump Procedure

```bash
# 1. Update workspace version
sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml

# 2. Update CHANGELOG.md (move [Unreleased] → [0.2.0])
# 3. Update docs references if needed

# 4. Commit
git add -A
git commit -m "chore: bump version to 0.2.0"

# 5. Tag
git tag -a v0.2.0 -m "Release v0.2.0 — Task Routing & Event Bus"

# 6. Push
git push origin main --tags
```

---

## 4. Git Tag Convention

```
v{MAJOR}.{MINOR}.{PATCH}[-{prerelease}]

Examples:
  v0.1.0-alpha.1
  v0.5.0-beta.2
  v1.0.0
  v1.0.1
```

### 4.1 Tag Rules

- Tags are **annotated** (not lightweight): `git tag -a v1.0.0 -m "..."`
- Tags are **GPG-signed** when possible: `git tag -s v1.0.0 -m "..."`
- Tags are **immutable** — never delete or move a published tag
- Tags trigger CI release pipeline → build artifacts → GitHub Release

---

## 5. Compatibility Matrix

### 5.1 Minimum Supported Rust Version (MSRV)

```
MSRV: 1.80.0
Policy: Bumped only in MINOR releases, announced in CHANGELOG
```

### 5.2 Platform Support

| Platform | Status | Tier |
|----------|--------|------|
| Linux x86_64 | ✅ Primary | Tier 1 (CI-tested, release binary) |
| Linux aarch64 | ✅ Primary | Tier 1 (CI-tested, release binary) |
| macOS x86_64 | ✅ Secondary | Tier 2 (CI-tested, release binary) |
| macOS aarch64 (Apple Silicon) | ✅ Secondary | Tier 2 (CI-tested, release binary) |
| Windows x86_64 | ⬜ Not supported (v1.0) | Tier 3 (community contributions welcome) |
| FreeBSD | ⬜ Not supported | Tier 3 |

### 5.3 Storage Backend Compatibility

| Backend | Minimum Version | Supported Since |
|---------|----------------|-----------------|
| SQLite | 3.35+ | v0.1.0 |
| PostgreSQL | 14+ | v0.5.0 (planned) |

### 5.4 SDK Compatibility

| SDK | Language Version | IAD Version |
|-----|-----------------|-------------|
| Python SDK | Python 3.10+ | v0.2.0+ |
| Node.js SDK | Node 18+ | v0.3.0+ |
| Rust SDK | Rust 1.80+ | v0.1.0+ |
| Go SDK | Go 1.21+ | v1.1.0+ (planned) |

---

## 6. Breaking Change Policy

### 6.1 Before v1.0.0
- Breaking changes can happen in any MINOR release
- All breaking changes documented in CHANGELOG with migration instructions
- Minimum 1 release warning before removal of features

### 6.2 After v1.0.0
- Breaking changes **only** in MAJOR releases
- Deprecated features kept for **at least 2 MINOR releases** before removal
- Deprecation warnings in CLI, logs, and docs
- Migration guide published with every MAJOR release
- LTS (Long-Term Support) for last MAJOR version — 12 months of security patches

### 6.3 What Constitutes a Breaking Change

| Breaking | Not Breaking |
|----------|-------------|
| CLI flag/command renamed or removed | New CLI command or flag added |
| REST API endpoint removed | New REST API endpoint added |
| Config key renamed or removed | New config key with default value |
| Agent manifest field renamed | New optional manifest field |
| Wire protocol change | New wire protocol message type |
| Storage schema migration (lossy) | Storage schema migration (lossless, automatic) |
| SDK method signature changed | New SDK method added |

---

## 7. Artifact Publishing

### 7.1 Release Artifacts

| Artifact | Location | Trigger |
|----------|----------|---------|
| GitHub Release | `github.com/rizoadev/ikamai-agent-daemon/releases` | Git tag `v*` |
| Binary (Linux x86_64) | GitHub Release assets | Git tag `v*` |
| Binary (Linux aarch64) | GitHub Release assets | Git tag `v*` |
| Binary (macOS x86_64) | GitHub Release assets | Git tag `v*` |
| Binary (macOS aarch64) | GitHub Release assets | Git tag `v*` |
| Docker Image | `ghcr.io/rizoadev/iad` | Git tag `v*` |
| Crates.io | `crates.io/crates/iad` | Git tag `v*` (stable only) |
| Checksum (SHA256) | GitHub Release assets | Git tag `v*` |

### 7.2 Docker Image Tags

```
ghcr.io/rizoadev/iad:latest          # Latest stable
ghcr.io/rizoadev/iad:1.0.0           # Specific version
ghcr.io/rizoadev/iad:1.0             # Latest patch for 1.0
ghcr.io/rizoadev/iad:1               # Latest minor for 1
ghcr.io/rizoadev/iad:0.5.0-beta.2    # Pre-release
ghcr.io/rizoadev/iad:nightly         # Nightly build from develop
```

---

## 8. Deprecation Process

```
v1.2.0  — Feature X marked @deprecated in docs, CLI shows warning
v1.3.0  — Feature X still works, warning louder ("will be removed in v2.0")
v1.4.0  — Feature X still works, but hidden from --help
v2.0.0  — Feature X removed, migration guide published
```

---

*Document maintained by Rizoa. Updated with each version bump.*
