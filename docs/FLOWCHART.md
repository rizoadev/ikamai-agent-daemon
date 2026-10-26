# Flowcharts & Diagrams

**Product:** IKAMAI Agent Daemon (IAD)  
**Date:** 2026-10-26

---

## 1. Daemon Startup Flow

```
                    ┌──────────┐
                    │ iad start│
                    └────┬─────┘
                         │
                    ┌────▼─────┐
                    │ Parse    │
                    │ CLI args │
                    └────┬─────┘
                         │
                    ┌────▼──────────┐
                    │ Load config   │
                    │ (iad.toml +   │
                    │  env vars)    │
                    └────┬──────────┘
                         │
                    ┌────▼──────────┐
                    │ Check PID file│
                    └────┬──────────┘
                         │
                ┌────────┴────────┐
                │                 │
           ┌────▼────┐      ┌────▼────────┐
           │ PID     │      │ PID stale/  │
           │ active  │      │ not found   │
           └────┬────┘      └────┬────────┘
                │                │
           ┌────▼────┐      ┌────▼─────────┐
           │ ABORT   │      │ Write PID    │
           │ (already│      │ file         │
           │ running)│      └────┬─────────┘
           └─────────┘           │
                            ┌────▼─────────┐
                            │ Init storage │
                            │ (SQLite)     │
                            │ Run migrations│
                            └────┬─────────┘
                                 │
                            ┌────▼─────────┐
                            │ Init services│
                            │ • Registry   │
                            │ • Router     │
                            │ • EventBus   │
                            │ • Scheduler  │
                            │ • Health Mon │
                            └────┬─────────┘
                                 │
                            ┌────▼──────────┐
                            │ Start API     │
                            │ (REST+WS+gRPC)│
                            └────┬──────────┘
                                 │
                            ┌────▼──────────┐
                            │ Resume agents │
                            │ (from DB)     │
                            └────┬──────────┘
                                 │
                            ┌────▼──────────┐
                            │ Resume        │
                            │ schedules     │
                            └────┬──────────┘
                                 │
                            ┌────▼──────────┐
                            │ ✅ RUNNING    │
                            │ (await signal)│
                            └───────────────┘
```

## 2. Agent Lifecycle Flow

```
                    ┌───────────┐
                    │ Manifest  │
                    │ submitted │
                    └─────┬─────┘
                          │
                    ┌─────▼──────┐
                    │ Validate   │
                    │ manifest   │
                    └─────┬──────┘
                          │
                ┌─────────┴─────────┐
                │                   │
           ┌────▼────┐         ┌────▼─────┐
           │ Invalid │         │ Valid    │
           │ → Error │         │          │
           └─────────┘         └────┬─────┘
                                    │
                              ┌─────▼──────┐
                              │ REGISTERED │
                              └─────┬──────┘
                                    │
                              ┌─────▼──────┐
                              │ iad agent  │
                              │ start      │
                              └─────┬──────┘
                                    │
                              ┌─────▼──────┐
                              │ STARTING   │
                              │ (spawn     │
                              │  process)  │
                              └─────┬──────┘
                                    │
                          ┌─────────┴─────────┐
                          │                   │
                     ┌────▼────┐         ┌────▼────┐
                     │ Spawn   │         │ Spawn   │
                     │ failed  │         │ success │
                     └────┬────┘         └────┬────┘
                          │                   │
                     ┌────▼────┐         ┌────▼────┐
                     │ FAILED  │         │ RUNNING │◄─── Restart
                     └────┬────┘         └────┬────┘         │
                          │                   │              │
                          │              ┌────▼────┐         │
                          │              │ Health  │         │
                          │              │ check   │         │
                          │              │ loop    │         │
                          │              └────┬────┘         │
                          │                   │              │
                          │         ┌─────────┴────────┐     │
                          │         │                  │     │
                          │    ┌────▼────┐        ┌────▼───┐ │
                          │    │ Healthy │        │ Failed │ │
                          │    │ → cont. │        │ (n++)  │ │
                          │    └─────────┘        └────┬───┘ │
                          │                            │     │
                          │                  ┌─────────┴──┐  │
                          │                  │            │  │
                          │             ┌────▼───┐  ┌─────▼──┴─┐
                          │             │ n >    │  │ n <=     │
                          │             │ max    │  │ max      │
                          │             └────┬───┘  └─────┬────┘
                          │                  │            │
                          │             ┌────▼───┐   ┌────▼────┐
                          ├◄────────────┤ FAILED │   │ RESTART │
                          │             │ (alert)│   │ (backoff│
                          │             └────────┘   └─────────┘
                          │
                     ┌────▼─────┐
                     │ iad agent│
                     │ stop     │
                     └────┬─────┘
                          │
                     ┌────▼──────┐
                     │ STOPPING  │
                     │ (SIGTERM) │
                     └────┬──────┘
                          │
                     ┌────▼──────┐    Timeout
                     │ Wait      ├────────────┐
                     │ graceful  │            │
                     └────┬──────┘       ┌────▼────┐
                          │              │ SIGKILL │
                     ┌────▼──────┐       └────┬────┘
                     │ STOPPED   │◄───────────┘
                     └───────────┘
```

## 3. Task Routing Flow

```
                    ┌───────────────┐
                    │ Task submitted│
                    │ {caps: [...]} │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ Validate task │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ Store in DB   │
                    │ status=pending│
                    └───────┬───────┘
                            │
                    ┌───────▼──────────────┐
                    │ Find agents matching │
                    │ required capabilities│
                    └───────┬──────────────┘
                            │
                  ┌─────────┴─────────┐
                  │                   │
             ┌────▼─────┐       ┌────▼──────┐
             │ No match │       │ Match(es) │
             │          │       │ found     │
             └────┬─────┘       └────┬──────┘
                  │                  │
             ┌────▼─────┐      ┌────▼──────────┐
             │ QUEUED   │      │ Select best   │
             │ (wait for│      │ (least busy,  │
             │  agent)  │      │  highest prio)│
             └──────────┘      └────┬──────────┘
                                    │
                              ┌─────▼─────────┐
                              │ ASSIGNED       │
                              │ → send to agent│
                              │ via IPC        │
                              └─────┬─────────┘
                                    │
                              ┌─────▼─────────┐
                              │ RUNNING        │
                              │ (agent working)│
                              └─────┬─────────┘
                                    │
                          ┌─────────┴─────────┐
                          │                   │
                     ┌────▼────┐         ┌────▼────┐
                     │ Success │         │ Failure │
                     └────┬────┘         └────┬────┘
                          │                   │
                     ┌────▼─────┐        ┌────▼──────┐
                     │COMPLETED │        │ Retry?    │
                     │ + result │        │ n < max?  │
                     └──────────┘        └────┬──────┘
                                              │
                                    ┌─────────┴────────┐
                                    │                  │
                               ┌────▼────┐        ┌───▼────┐
                               │ Yes     │        │ No     │
                               │ → retry │        │ →FAILED│
                               │ (backoff│        │ +error │
                               └────┬────┘        └────────┘
                                    │
                               ┌────▼─────┐
                               │ Re-queue │
                               │ (pending)│
                               └──────────┘
```

## 4. Event Bus Flow

```
                              ┌──────────┐
                              │ Agent A  │
                              │ publishes│
                              │ event    │
                              └────┬─────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │         EVENT BUS            │
                    │                              │
                    │  Topic: "data.processed"     │
                    │  ┌─────────────────────────┐ │
                    │  │ Subscribers:            │ │
                    │  │  • Agent B (filter: *)  │ │
                    │  │  • Agent C (filter:     │ │
                    │  │    type=csv)            │ │
                    │  │  • WebSocket client     │ │
                    │  └─────────────────────────┘ │
                    │                              │
                    │  1. Persist event (SQLite)    │
                    │  2. Match subscribers         │
                    │  3. Apply filters             │
                    │  4. Deliver to matched        │
                    └──┬──────────┬──────────┬──────┘
                       │          │          │
                  ┌────▼───┐ ┌───▼────┐ ┌───▼────┐
                  │Agent B │ │Agent C │ │WebSocket│
                  │(match) │ │(match) │ │client   │
                  └────────┘ └────────┘ └─────────┘
```

## 5. Scheduler Flow

```
                    ┌────────────────────┐
                    │ Scheduler Service  │
                    │ (runs every 1s)    │
                    └────────┬───────────┘
                             │
                    ┌────────▼────────────┐
                    │ Query schedules     │
                    │ WHERE next_run <= now│
                    │ AND enabled = true  │
                    └────────┬────────────┘
                             │
                   ┌─────────┴─────────┐
                   │                   │
              ┌────▼────┐         ┌────▼────┐
              │ No jobs │         │ Jobs    │
              │ → sleep │         │ found   │
              └─────────┘         └────┬────┘
                                       │
                                  ┌────▼──────────┐
                                  │ For each job: │
                                  │ 1. Submit task│
                                  │    from       │
                                  │    template   │
                                  │ 2. Update     │
                                  │    last_run   │
                                  │ 3. Calculate  │
                                  │    next_run   │
                                  └────┬──────────┘
                                       │
                                  ┌────▼──────┐
                                  │ Cron?     │
                                  └────┬──────┘
                                       │
                             ┌─────────┴────────┐
                             │                  │
                        ┌────▼─────┐       ┌────▼─────┐
                        │ Yes:     │       │ No:      │
                        │ calc     │       │ one-shot │
                        │ next_run │       │ disable  │
                        └──────────┘       └──────────┘
```

## 6. Graceful Shutdown Flow

```
                    ┌───────────────┐
                    │ SIGTERM/SIGINT│
                    │ received     │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ Set shutdown  │
                    │ flag          │
                    └───────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
        ┌─────▼────┐  ┌────▼─────┐  ┌────▼─────┐
        │ Stop API │  │ Stop     │  │ Drain    │
        │ listener │  │ accepting│  │ event    │
        │ (no new  │  │ new tasks│  │ queue    │
        │ conns)   │  │          │  │          │
        └─────┬────┘  └────┬─────┘  └────┬─────┘
              │             │             │
              └─────────────┼─────────────┘
                            │
                    ┌───────▼───────┐
                    │ Wait for      │
                    │ in-flight     │
                    │ tasks (30s    │
                    │ timeout)      │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ Stop all      │
                    │ agents        │
                    │ (SIGTERM →    │
                    │  SIGKILL)     │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ Flush storage │
                    │ (WAL          │
                    │ checkpoint)   │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ Remove PID    │
                    │ file          │
                    └───────┬───────┘
                            │
                    ┌───────▼───────┐
                    │ EXIT 0        │
                    └───────────────┘
```

## 7. Multi-Node Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                        CONTROL PLANE                          │
│                                                               │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│   │   Node A     │     │   Node B     │     │   Node C     │  │
│   │   (Primary)  │     │   (Worker)   │     │   (Worker)   │  │
│   │              │     │              │     │              │  │
│   │  ┌────────┐  │     │  ┌────────┐  │     │  ┌────────┐  │  │
│   │  │ IAD    │◄─┼─gRPC─┼─►│ IAD    │◄─┼─gRPC─┼─►│ IAD    │  │  │
│   │  │ daemon │  │     │  │ daemon │  │     │  │ daemon │  │  │
│   │  └───┬────┘  │     │  └───┬────┘  │     │  └───┬────┘  │  │
│   │      │       │     │      │       │     │      │       │  │
│   │  ┌───┴───┐   │     │  ┌───┴───┐   │     │  ┌───┴───┐   │  │
│   │  │Agents │   │     │  │Agents │   │     │  │Agents │   │  │
│   │  │A1  A2 │   │     │  │B1  B2 │   │     │  │C1  C2 │   │  │
│   │  │A3     │   │     │  │B3  B4 │   │     │  │C3     │   │  │
│   │  └───────┘   │     │  └───────┘   │     │  └───────┘   │  │
│   │              │     │              │     │              │  │
│   │  SQLite      │     │  SQLite      │     │  SQLite      │  │
│   └──────────────┘     └──────────────┘     └──────────────┘  │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐     │
│   │              Shared PostgreSQL                       │     │
│   │        (when multi-node enabled)                     │     │
│   └─────────────────────────────────────────────────────┘     │
└───────────────────────────────────────────────────────────────┘

Data Flow:
  Task submit → Primary → Route to best node → Agent on that node
  Event publish → Source node → Propagate to all nodes → Local delivery
  Health → Each node monitors its agents → Report to Primary
```

## 8. SDK Communication Flow

```
┌─────────────────┐          ┌─────────────────┐
│   Agent Process  │          │   IAD Daemon     │
│                  │          │                  │
│  ┌────────────┐  │  Unix    │  ┌────────────┐  │
│  │ Agent SDK  │◄─┼──Socket──┼─►│ IPC Server │  │
│  └──────┬─────┘  │          │  └──────┬─────┘  │
│         │        │          │         │        │
│  ┌──────▼─────┐  │          │  ┌──────▼─────┐  │
│  │ User Code  │  │          │  │ Services   │  │
│  │            │  │          │  │            │  │
│  │ @on_task   │  │  ◄────── │  │ Router     │  │
│  │ @on_event  │  │  task    │  │ EventBus   │  │
│  │ publish()  │  │  ──────► │  │ Registry   │  │
│  │ heartbeat()│  │  event   │  │ Health     │  │
│  └────────────┘  │          │  └────────────┘  │
└─────────────────┘          └─────────────────┘

Wire Protocol (length-prefixed JSON over Unix socket):

  [4 bytes: payload length (big-endian u32)]
  [N bytes: JSON payload]

  Agent → Daemon:
    {"type":"register","caps":["text-gen"]}
    {"type":"heartbeat","ts":"2026-10-26T..."}
    {"type":"task_result","task_id":"...","result":{...}}
    {"type":"publish","topic":"x","payload":{...}}

  Daemon → Agent:
    {"type":"registered","agent_id":"..."}
    {"type":"task","id":"...","payload":{...}}
    {"type":"event","topic":"x","payload":{...}}
    {"type":"health_check"}
    {"type":"shutdown","grace_period_ms":5000}
```

---

*Diagrams maintained by Rizoa. ASCII art for maximum portability.*
