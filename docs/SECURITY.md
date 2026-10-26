# IAD Security Guide

## Threat Model

| Threat | Mitigation |
|--------|-----------|
| Unauthorized API access | Bearer token auth (RBAC in Phase 4.11) |
| Agent code injection | WASM sandbox (Phase 4.9), process isolation |
| Secret leakage | Secret store with env injection (Phase 4.12) |
| Resource exhaustion | cgroups v2 CPU/memory limits (Phase 4.13) |
| PID file race | flock() — TOCTOU-safe (plan D6) |
| Orphan processes | Process groups, PR_SET_PDEATHSIG (plan D7) |
| Replay attacks | ULID task IDs (monotonic, unique) |
| Retry storms | Global retry budget + circuit breaker (plan 2.R1) |
| Task queue overflow | Bounded queue + HTTP 429 backpressure (plan 2.4) |

## Running `cargo audit`

```bash
cargo audit
```

## Known Security TODOs

- [ ] Replace bearer tokens with mTLS (Phase 4.10)
- [ ] Implement AES-256-GCM encryption for secret vault
- [ ] Add rate limiting middleware to API
- [ ] Add IP allowlist for API access
- [ ] Implement token rotation / expiry

## Reporting Vulnerabilities

Email: security@ikamai.id

Do NOT create public GitHub issues for security vulnerabilities.
