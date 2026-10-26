# Roadmap & Timeline

**Product:** IKAMAI Agent Daemon (IAD)  
**Date:** 2026-10-26  
**Author:** Rizoa

---

## Visual Roadmap

```
2026                                               2027
Oct   Nov   Dec   Jan   Feb   Mar   Apr   May   Jun   Jul
 │     │     │     │     │     │     │     │     │     │
 ├──P0─┤     │     │     │     │     │     │     │     │
 │ Foundation │     │     │     │     │     │     │     │
 │     ├──P1──┤     │     │     │     │     │     │     │
 │     │ Core │     │     │     │     │     │     │     │
 │     │ Daemon     │     │     │     │     │     │     │
 │     │     ├──P2──┤     │     │     │     │     │     │
 │     │     │ Task/│     │     │     │     │     │     │
 │     │     │ Event│     │     │     │     │     │     │
 │     │     │     ├────P3────┤     │     │     │     │
 │     │     │     │  API &   │     │     │     │     │
 │     │     │     │  Monitor │     │     │     │     │
 │     │     │     │     │     ├────P4────┤     │     │
 │     │     │     │     │     │  Scale & │     │     │
 │     │     │     │     │     │  Plugins │     │     │
 │     │     │     │     │     │     │     ├────P5────┤
 │     │     │     │     │     │     │     │Production│
 │     │     │     │     │     │     │     │     │ v1.0│
 │     │     │     │     │     │     │     │     │     │
 ▼     ▼     ▼     ▼     ▼     ▼     ▼     ▼     ▼     ▼

Releases:
 v0.1.0-alpha.1 ──┐
                   v0.2.0-alpha.2 ──┐
                                    v0.3.0-beta.1 ──┐
                                                    v0.5.0-beta.2 ──┐
                                                                    v1.0.0 ──►
```

## Milestones

### M1: First Heartbeat (v0.1.0-alpha.1) — End of November 2026
- [x] Project scaffolded + CI green
- [ ] Daemon starts as background process
- [ ] Agent manifest parsed and validated
- [ ] Agents start/stop/restart via CLI
- [ ] Health check with auto-restart
- [ ] SQLite persistent state
- [ ] Structured logging
- **Demo:** `iad start && iad agent add hello.yaml && iad agent list`

### M2: Agents Talk (v0.2.0-alpha.2) — End of December 2026
- [ ] IPC protocol over Unix sockets
- [ ] Task submission and capability-based routing
- [ ] Task lifecycle + retry
- [ ] Event bus (pub/sub)
- [ ] Cron scheduler
- [ ] Python SDK (basic)
- **Demo:** Submit task → routed to capable agent → result returned

### M3: Control Plane (v0.3.0-beta.1) — End of February 2027
- [ ] Full REST API (Axum)
- [ ] WebSocket real-time events
- [ ] API authentication
- [ ] Prometheus metrics
- [ ] Alert webhooks
- [ ] Hot-reload config
- **Demo:** `curl localhost:7700/api/v1/agents` + Grafana dashboard

### M4: Fleet Ready (v0.5.0-beta.2) — End of April 2027
- [ ] gRPC inter-daemon communication
- [ ] Multi-node agent management
- [ ] Cross-node task routing
- [ ] WASM plugin runtime
- [ ] PostgreSQL backend
- [ ] mTLS + RBAC
- **Demo:** 3-node fleet managing 30 agents, cross-node task routing

### M5: Production Launch (v1.0.0) — End of June 2027
- [ ] All docs complete
- [ ] Benchmark: 100 agents < 50MB overhead
- [ ] Security audit passed
- [ ] Docker + systemd deployment
- [ ] 5 example agents
- [ ] Landing page live
- **Launch:** GitHub release + blog post + HN/Reddit post

## Post-v1.0 Roadmap

| Quarter | Version | Focus |
|---------|---------|-------|
| Q3 2027 | v1.1 | Web dashboard, more SDKs (Go, Node) |
| Q4 2027 | v1.2 | Agent marketplace, templates |
| Q1 2028 | v1.3 | Enterprise features (SSO, audit, compliance) |
| Q2 2028 | v2.0 | Breaking: new plugin API, distributed consensus |

## Key Dates

| Date | Event |
|------|-------|
| 2026-10-26 | Project kickoff, docs complete |
| 2026-11-01 | GitHub repo public, CI green |
| 2026-11-30 | v0.1.0-alpha.1 release |
| 2026-12-31 | v0.2.0-alpha.2 release |
| 2027-02-28 | v0.3.0-beta.1 release |
| 2027-04-30 | v0.5.0-beta.2 release |
| 2027-06-30 | v1.0.0 GA release 🚀 |

---

*Roadmap maintained by Rizoa. Reviewed monthly.*
