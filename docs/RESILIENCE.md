# Resilience & Fault Handling Specification

**Product:** IKAMAI Agent Daemon (IAD)  
**Version:** 0.1.0  
**Date:** 2026-10-27  
**Author:** Rizoa  
**Status:** ACTIVE — Must be addressed before v0.1.0  
**Review:** Gemini 3.8 Flash + Claude analysis  

---

## Overview

IAD is a long-running daemon managing AI agent processes. In production, everything
that can fail **will** fail. This document specifies how IAD handles every failure mode —
from kernel OOM kills to LLM mid-stream disconnects to cascading retry storms.

**Design Principle:** _Assume crash is normal, not exceptional._

---

## 1. Process-Level Fault Handling

### 1.1 Zombie Process Reaping

**Problem:** When an agent child process exits, if the daemon doesn't call `waitpid()`,
the process becomes a zombie, leaking kernel resources indefinitely.

**Solution:**
```rust
// Dedicated SIGCHLD handler + async waitpid via pidfd
// Every spawned child MUST be tracked in ProcessTable
use tokio::signal::unix::{signal, SignalKind};

async fn reaper_loop(process_table: Arc<RwLock<ProcessTable>>) {
    let mut sigchld = signal(SignalKind::child()).unwrap();
    loop {
        sigchld.recv().await;
        let mut table = process_table.write().await;
        table.reap_zombies().await; // waitpid(WNOHANG) loop
    }
}
```

**Requirements:**
- [ ] `SIGCHLD` handler runs on dedicated Tokio task
- [ ] `waitpid(-1, WNOHANG)` in a loop until no more children
- [ ] Extract exit code + signal from `WaitStatus`
- [ ] Log exit reason to `agent_incidents` table
- [ ] Trigger restart policy evaluation after reap

### 1.2 Orphan Process Adoption

**Problem:** Agent spawns child processes (e.g., Python subprocess, ffmpeg). If the
agent dies, these grandchildren become orphaned under PID 1, invisible to IAD.

**Solution: Process Group + `PR_SET_PDEATHSIG` + cgroups v2**
```rust
use std::os::unix::process::CommandExt;

let mut cmd = tokio::process::Command::new(&entrypoint);
cmd.process_group(0); // New PGID = child PID
unsafe {
    cmd.pre_exec(|| {
        // Tell kernel: kill me when my parent dies
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
        Ok(())
    });
}
```

**Requirements:**
- [ ] Every agent runs in its own **process group** (`setpgid(0, 0)`)
- [ ] `PR_SET_PDEATHSIG(SIGTERM)` set via `pre_exec`
- [ ] Stop/kill sends signal to **negative PID** (entire group): `kill(-pgid, sig)`
- [ ] Phase 4+: Use **cgroups v2** scope for guaranteed kill (`cgroup.kill`)
- [ ] After group kill, verify no leftover processes via `/proc` scan

### 1.3 PID File Race Condition (flock)

**Problem:** Checking PID file → reading → checking if alive → writing has TOCTOU race.
Two daemons can start simultaneously and both think they're the only one.

**Solution: Advisory flock on PID file**
```rust
use std::fs::OpenOptions;
use nix::fcntl::{flock, FlockArg};

fn acquire_pid_lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(path)?;
    
    // Non-blocking exclusive lock
    flock(file.as_raw_fd(), FlockArg::LockExclusiveNonblock)
        .map_err(|_| Error::DaemonAlreadyRunning)?;
    
    // Truncate and write our PID
    file.set_len(0)?;
    write!(file, "{}", std::process::id())?;
    file.sync_all()?;
    
    Ok(file) // MUST keep file open — lock released on drop/close
}
```

**Requirements:**
- [ ] `flock(LOCK_EX | LOCK_NB)` on PID file
- [ ] File handle held for daemon lifetime (never close until shutdown)
- [ ] Graceful shutdown: remove PID file, then close (releases lock)
- [ ] Crash: lock auto-released by kernel on process exit

### 1.4 File Descriptor Leak Prevention

**Problem:** Every agent restart creates new Unix sockets, pipes for stdout/stderr,
and log file handles. Over weeks of operation, FDs leak if not properly closed.

**Solution: FD tracking + CLOEXEC + periodic audit**
```rust
// All file opens use CLOEXEC by default
// Track open FDs per agent in AgentProcess struct

pub struct AgentProcess {
    pid: u32,
    pidfd: OwnedFd,
    stdout_pipe: Option<OwnedFd>,
    stderr_pipe: Option<OwnedFd>,
    socket: Option<OwnedFd>,
    // All Drop'd on agent stop
}

// Periodic FD audit
async fn fd_audit(pid: u32) -> usize {
    let fd_dir = format!("/proc/{}/fd", pid);
    std::fs::read_dir(&fd_dir)
        .map(|entries| entries.count())
        .unwrap_or(0)
}
```

**Requirements:**
- [ ] All FDs opened with `O_CLOEXEC` (Rust default for `File::open`)
- [ ] `AgentProcess` struct owns all FDs, implements `Drop` for cleanup
- [ ] Daemon self-monitors its own FD count every 60s
- [ ] Alert if daemon FD usage > 80% of `RLIMIT_NOFILE`
- [ ] Set per-agent `RLIMIT_NOFILE` via `prlimit` before spawn
- [ ] Prometheus gauge: `iad_daemon_fd_count`, `iad_agent_fd_count{agent}`

### 1.5 PID Recycling Safety (pidfd)

**Problem:** On Linux, PIDs recycle fast (default max 32768). After agent dies, its PID
could be reused by an unrelated process. Sending `SIGKILL` to that PID kills an innocent process.

**Solution: Use `pidfd_open(2)` (Linux 5.3+)**
```rust
use std::os::unix::io::{FromRawFd, OwnedFd};
use tokio::io::unix::AsyncFd;

pub fn open_pidfd(pid: u32) -> std::io::Result<OwnedFd> {
    let fd = unsafe {
        libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0_u32)
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd as i32) })
}

// Signal via pidfd instead of PID
pub fn pidfd_kill(pidfd: &OwnedFd, sig: i32) -> std::io::Result<()> {
    let ret = unsafe {
        libc::syscall(libc::SYS_pidfd_send_signal, pidfd.as_raw_fd(), sig, std::ptr::null::<libc::siginfo_t>(), 0_u32)
    };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
```

**Requirements:**
- [ ] Always open `pidfd` immediately after `fork/spawn`
- [ ] Never use raw PID for signaling — use `pidfd_send_signal(2)`
- [ ] Wait for exit via `pidfd` poll readability (no SIGCHLD race)
- [ ] Fallback to traditional PID on kernels < 5.3 (log warning)

---

## 2. Daemon-Level Fault Handling

### 2.1 Daemon Crash Recovery

**Scenario:** Server reboots, OOM-killer targets daemon, power loss.

**Recovery sequence on restart:**
```
1. Acquire PID file flock (abort if another instance running)
2. Open SQLite with WAL integrity check
3. Load all agents with status = 'running' or 'starting'
4. For each:
   a. Check if PID still alive AND pidfd valid
   b. If alive: re-attach monitoring (adopt)
   c. If dead: set status='crashed', evaluate restart_policy
5. Scan for orphaned Unix sockets in run/ directory — clean up
6. Reclaim expired task leases (lease_expires_at < now)
7. Move reclaimed tasks → 'pending' (if retry_count < max_retries) or 'dead_letter'
8. Resume scheduler (recalculate next_run for missed fires)
9. Replay undelivered events from sequence gaps
10. Emit event: daemon.recovered { recovered_agents, reclaimed_tasks }
```

**Requirements:**
- [ ] WAL integrity check on startup: `PRAGMA integrity_check`
- [ ] Agent status `crashed` distinct from `stopped` and `failed`
- [ ] Generation counter to detect state staleness (see §4.1)
- [ ] Recovery time < 5 seconds for 100 agents

### 2.2 Daemon OOM Prevention

**Problem:** Daemon itself gets OOM-killed. Everything goes down.

**Solution:**
- [ ] Set `OOM_SCORE_ADJ=-500` for daemon process (less likely to be killed)
- [ ] Set `OOM_SCORE_ADJ=+300` for agent processes (more likely to be killed first)
- [ ] Monitor daemon RSS via `/proc/self/statm`
- [ ] Bounded internal buffers everywhere (no unbounded channels)
- [ ] Emergency mode: if RSS > 80% of limit, stop accepting new tasks

```rust
fn set_oom_score(pid: u32, score: i32) -> std::io::Result<()> {
    std::fs::write(
        format!("/proc/{}/oom_score_adj", pid),
        score.to_string()
    )
}
```

### 2.3 Thundering Herd on Restart

**Problem:** After daemon restart, all crashed agents restart simultaneously,
causing CPU/memory spike and potential cascade failure.

**Solution: Staggered restart with jitter**
```rust
async fn restart_agents_staggered(agents: Vec<Agent>) {
    let mut rng = rand::thread_rng();
    let batch_size = 5; // Max concurrent restarts
    
    for chunk in agents.chunks(batch_size) {
        let futures: Vec<_> = chunk.iter().map(|agent| {
            let jitter = Duration::from_millis(rng.gen_range(100..2000));
            async move {
                tokio::time::sleep(jitter).await;
                agent.restart().await
            }
        }).collect();
        
        join_all(futures).await;
        tokio::time::sleep(Duration::from_secs(3)).await; // Inter-batch cooldown
    }
}
```

**Requirements:**
- [ ] Max 5 concurrent agent restarts
- [ ] Random jitter 100ms–2s per agent
- [ ] 3s cooldown between batches
- [ ] Priority-based ordering (urgent agents first)
- [ ] Configurable via `[daemon] max_concurrent_restarts`

---

## 3. Agent Communication Fault Handling

### 3.1 Unix Socket Read/Write Timeout

**Problem:** Agent hangs, stops reading from socket. Daemon's write blocks forever.
Or agent sends partial frame (Slowloris-style), tying up a Tokio task.

**Solution:**
```rust
use tokio::time::timeout;

const FRAME_READ_TIMEOUT: Duration = Duration::from_secs(30);
const FRAME_WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MB

async fn read_frame(stream: &mut UnixStream) -> Result<Frame> {
    // Timeout on reading the length prefix
    let len_bytes = timeout(FRAME_READ_TIMEOUT, read_exact(stream, 4)).await
        .map_err(|_| Error::ReadTimeout)??;
    
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(Error::FrameTooLarge(len));
    }
    
    // Timeout on reading the payload
    let payload = timeout(FRAME_READ_TIMEOUT, read_exact(stream, len)).await
        .map_err(|_| Error::ReadTimeout)??;
    
    Ok(Frame::parse(&payload)?)
}
```

**Requirements:**
- [ ] 30s read timeout per frame
- [ ] 10s write timeout per frame
- [ ] 16 MB max frame size (reject larger with error frame)
- [ ] Idle timeout: disconnect agent socket after 5min no heartbeat
- [ ] Per-agent socket buffer monitoring

### 3.2 Agent Heartbeat & Lease System

**Problem:** Agent appears alive (process running) but is functionally dead —
stuck in infinite loop, deadlocked, waiting on network forever.

**Solution: Two-tier liveness**
```
Tier 1 — Process Heartbeat (is the process alive?)
  - Daemon sends PING frame every 30s
  - Agent MUST reply PONG within 10s
  - 3 missed → mark unhealthy → evaluate restart

Tier 2 — Task Lease (is the agent making progress?)
  - Each task assigned with lease_token + lease_expires_at (TTL)
  - Agent MUST send heartbeat_lease(task_id, lease_token) every 10s
  - Expired lease → task reclaimed → reassigned or dead-lettered
```

**Requirements:**
- [ ] PING/PONG frames in wire protocol
- [ ] Task lease with cryptographic token (prevent stale agent from reporting)
- [ ] Lease TTL configurable per-task (default: 5min)
- [ ] Agent generation_id check: ignore results from previous incarnation

### 3.3 Backpressure on Task Queue

**Problem:** Tasks submitted faster than agents can process. Unbounded queue
grows until daemon OOMs.

**Solution: Bounded queue + admission control**
```rust
// Bounded channel with configurable capacity
let (tx, rx) = tokio::sync::mpsc::channel::<Task>(10_000); // Global cap

// Per-agent bounded send
match agent_tx.try_send(task) {
    Ok(_) => {},
    Err(TrySendError::Full(_)) => {
        // Option A: Reject with 429 (backpressure to client)
        // Option B: Enqueue to overflow/persistent queue
        // Option C: Drop lowest priority task
    }
}
```

**Requirements:**
- [ ] Global task queue: bounded at 10,000 (configurable)
- [ ] Per-agent in-flight limit: `max_concurrent_tasks` from manifest
- [ ] HTTP 429 response when queue full
- [ ] Prometheus gauge: `iad_task_queue_depth`, `iad_task_queue_capacity`
- [ ] Alert when queue > 80% capacity
- [ ] Priority queue: urgent tasks preempt low-priority

### 3.4 Retry Storm Prevention

**Problem:** Agent keeps failing. Exponential backoff helps, but if 50 agents
all fail at once, even staggered retries create a storm.

**Solution: Global retry budget + circuit breaker**
```rust
pub struct RetryBudget {
    tokens: AtomicU32,       // Available retry tokens
    max_tokens: u32,         // Bucket size (e.g., 100)
    refill_rate: u32,        // Tokens per second (e.g., 10)
    last_refill: AtomicU64,  // Epoch millis
}

impl RetryBudget {
    pub fn try_acquire(&self) -> bool {
        self.refill(); // Add tokens based on elapsed time
        self.tokens.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            if current > 0 { Some(current - 1) } else { None }
        }).is_ok()
    }
}
```

**Per-agent circuit breaker states:**
```
Closed (normal) → 5 consecutive failures → Open (reject all tasks)
Open → cooldown 60s → Half-Open (allow 1 probe task)  
Half-Open → probe succeeds → Closed
Half-Open → probe fails → Open (reset cooldown)
```

**Requirements:**
- [ ] Per-agent exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s (cap)
- [ ] Global retry budget: 100 tokens, refill 10/s
- [ ] Circuit breaker per agent: threshold=5, cooldown=60s
- [ ] Jitter on all retry delays (±25%)
- [ ] Prometheus counter: `iad_retries_total{agent, outcome}`
- [ ] Prometheus gauge: `iad_circuit_breaker_state{agent}` (0=closed, 1=open, 2=half-open)

### 3.5 WebSocket Session Resilience

**Problem:** Client WebSocket drops mid-stream. Agent keeps processing (wasting LLM tokens).
Or client reconnects and misses events that occurred during disconnect.

**Solution:**
```
Disconnect detected:
  1. Mark session as disconnected (keep state for 5min)
  2. Send CANCEL_TASK to agent via Unix socket (if streaming)
  3. Agent propagates cancellation to LLM API (CancellationToken)
  4. Buffer events for session (up to 1000 events or 5min)

Reconnect:
  1. Client sends last_event_sequence
  2. Daemon replays missed events from buffer/SQLite
  3. Resume streaming if task still running
```

**Requirements:**
- [ ] WebSocket ping/pong every 30s
- [ ] Disconnect grace period: 5min (configurable)
- [ ] Event replay on reconnect (sequence-based)
- [ ] Cancellation propagation: WS disconnect → Unix socket `CANCEL_TASK` frame
- [ ] Max buffered events per session: 1000

---

## 4. Data Integrity & Storage Fault Handling

### 4.1 SQLite Corruption Prevention

**Problem:** Power loss during WAL write, disk full, concurrent access bugs.

**Solution:**
```sql
-- On every connection
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;    -- Good balance of safety vs speed
PRAGMA busy_timeout = 5000;     -- Wait 5s for locks instead of SQLITE_BUSY
PRAGMA wal_autocheckpoint = 1000; -- Checkpoint every 1000 pages

-- Periodic maintenance (every 24h)
PRAGMA integrity_check;
PRAGMA optimize;
```

**Requirements:**
- [ ] WAL mode always enabled
- [ ] Busy timeout 5s (avoid SQLITE_BUSY errors)
- [ ] Auto-checkpoint every 1000 pages
- [ ] Manual checkpoint on graceful shutdown
- [ ] Integrity check on startup + daily
- [ ] WAL file size monitoring: alert if > 100MB
- [ ] Disk space check before write-heavy operations
- [ ] Read-only fallback mode if disk < 5% free
- [ ] Backup: daily copy of DB file (during checkpoint)

### 4.2 State Consistency via Generation Counter

**Problem:** After crash recovery, stale agent might report results for tasks that
have been reassigned.

**Solution:**
```sql
ALTER TABLE agents ADD COLUMN generation_id INTEGER NOT NULL DEFAULT 1;
```

```rust
// On agent restart, increment generation
fn restart_agent(agent: &mut Agent) {
    agent.generation_id += 1;
    // Store in DB
}

// On task result from agent, verify generation
fn handle_task_result(agent_id: &str, gen_id: u64, result: TaskResult) {
    let current = db.get_agent_generation(agent_id)?;
    if gen_id != current {
        warn!("Stale result from agent {} (gen {} vs current {})", 
              agent_id, gen_id, current);
        return; // Discard
    }
    // Accept result
}
```

### 4.3 Data Retention Policy

**Problem:** Events, task results, health ticks, and logs grow unbounded.

**Retention rules:**
| Data Type | Retention | Action |
|-----------|-----------|--------|
| Completed tasks | 30 days | Archive to cold storage, then delete |
| Failed tasks | 90 days | Keep for debugging |
| Dead-letter tasks | 90 days | Manual review required |
| Events | 7 days | Delete (compact topic sequences) |
| Health ticks | 24 hours | Rolling window delete |
| Agent incidents | 180 days | Keep for post-mortems |
| Daemon logs | 30 days | Rotate via logrotate |
| Agent logs | 7 days | Rotate per agent |

**Requirements:**
- [ ] Background janitor task running every hour
- [ ] Configurable retention per data type in `iad.toml`
- [ ] Soft-delete before hard-delete (tombstone period: 7 days)
- [ ] Vacuum after bulk deletes (reclaim disk)
- [ ] Prometheus gauge: `iad_db_size_bytes`, `iad_wal_size_bytes`

---

## 5. AI-Agent-Specific Fault Handling

### 5.1 LLM Response Timeout & Mid-Stream Failure

**Scenario:** Agent calls OpenAI API, response streams 450 tokens then connection drops.
Network timeout. Rate limit hit. Server error.

**Handling in agent manifest:**
```yaml
llm:
  response_timeout: "120s"           # Max time waiting for complete response
  stream_idle_timeout: "30s"         # Max gap between streaming chunks
  max_retries_on_transient: 3        # Retry on 429, 500, 502, 503
  retry_backoff: "exponential"       # 1s, 2s, 4s
```

**Daemon-side enforcement:**
```
Agent sends: { "type": "task_progress", "task_id": "...", "tokens_so_far": 450 }
Daemon tracks: last_progress_at per task
If no progress for stream_idle_timeout → send CANCEL_TASK → reclaim
```

**Error Classification:**
```rust
pub enum AgentError {
    // Retryable — daemon will retry
    Transient(TransientError),     // Network timeout, 429, 500, 502, 503
    
    // Non-retryable — daemon will NOT retry, moves to dead-letter
    Fatal(FatalError),             // 400, 401, 403, context overflow, invalid input
    
    // Budget exceeded — daemon isolates agent
    BudgetExceeded(BudgetError),   // Token limit, cost limit
}
```

**Requirements:**
- [ ] SDK provides `report_progress(task_id, tokens)` method
- [ ] Daemon monitors progress updates per task
- [ ] Error frame includes `retryable: bool` field
- [ ] Non-retryable errors skip retry, go straight to dead-letter
- [ ] Progress timeout distinct from total task timeout

### 5.2 Hallucination Loop / Tool Ping-Pong

**Problem:** Agent LLM calls tool A → fails → calls tool B → calls tool A again in
infinite loop, burning tokens and compute.

**Solution: Depth + cycle detection**
```yaml
# Agent manifest
limits:
  max_tool_calls_per_task: 25       # Hard cap on tool invocations
  max_tool_depth: 10                # Max nested tool chain depth
  total_execution_timeout: "300s"   # Hard cap on task duration
  max_identical_calls: 3            # Same tool+args = cycle detected
```

**Requirements:**
- [ ] Task metadata tracks call chain depth
- [ ] Daemon counts tool calls per task
- [ ] Detect identical consecutive calls (same tool + same args hash)
- [ ] Auto-cancel with `TOOL_LOOP_DETECTED` status
- [ ] Log full call chain for debugging

### 5.3 Cost Runaway Prevention

**Problem:** Agent sends requests to expensive LLM models without limits.
$1000 bill before anyone notices.

**Solution: Budget guardrails in manifest**
```yaml
quotas:
  max_cost_per_task_usd: 1.00       # Per-task limit
  max_cost_per_hour_usd: 10.00      # Hourly rolling limit
  max_cost_per_day_usd: 50.00       # Daily limit
  max_tokens_per_task: 32000         # Input + output tokens
  max_tokens_per_hour: 500000        # Hourly rolling limit
```

**Enforcement:**
```
Agent sends: { "type": "usage_report", "tokens_in": 1200, "tokens_out": 800, "cost_usd": 0.05 }
Daemon accumulates per agent in rolling windows
If limit exceeded → isolate agent → reject new tasks → alert
```

**Requirements:**
- [ ] SDK provides `report_usage(tokens_in, tokens_out, cost)` method
- [ ] Daemon tracks rolling 1h and 24h windows
- [ ] Soft limit (80%): warn via alert
- [ ] Hard limit (100%): isolate agent, reject tasks
- [ ] Budget reset on new window (not on restart)
- [ ] Prometheus: `iad_agent_cost_usd{agent}`, `iad_agent_tokens_total{agent, direction}`

### 5.4 Context Window Exhaustion

**Problem:** Agent feeds growing conversation to LLM until context limit hit.
Gets 400 error, retries same oversized request in loop.

**Solution:**
- Agent SDK MUST classify errors (see §5.1 Error Classification)
- HTTP 400 from LLM provider → `Fatal(ContextOverflow)` → non-retryable
- Daemon moves task to dead-letter with `CONTEXT_OVERFLOW` reason
- Alert: "Agent X hit context limit on task Y — consider chunking"

### 5.5 Cancellation Propagation

**Problem:** Client cancels task, but agent is mid-request to LLM API.
Tokens still consumed, response discarded.

**Solution: Cooperative cancellation chain**
```
Client HTTP DELETE /tasks/:id
  → Daemon sets task status = 'cancelling'
  → Daemon sends CANCEL_TASK frame to agent via Unix socket
  → Agent receives cancel, drops CancellationToken
  → Agent's HTTP client (reqwest) aborts LLM request
  → Agent sends { "type": "task_cancelled", "task_id": "...", "tokens_consumed": 200 }
  → Daemon marks task 'cancelled', records partial cost
```

**Requirements:**
- [ ] `CANCEL_TASK` frame type in wire protocol
- [ ] SDK `CancellationToken` passed to task handler
- [ ] Agent MUST check token periodically during long operations
- [ ] Daemon waits max 5s for graceful cancel, then kills task context
- [ ] Partial results preserved if `save_partial: true` in task config

---

## 6. Network & Infrastructure Fault Handling

### 6.1 Server Reboot Recovery

**Scenario:** Unexpected reboot (power failure, kernel panic, cloud instance preemption).

**Sequence:**
```
1. systemd starts iad.service (Restart=always)
2. Daemon runs recovery sequence (§2.1)
3. All agents with restart_policy != "never" are restarted (staggered, §2.3)
4. Missed cron jobs evaluated per missed_fire_policy
5. Pending tasks with expired leases reclaimed
6. WebSocket clients must reconnect (no state preserved across reboot)
7. Emit alert: "IAD recovered from unexpected shutdown"
```

**systemd hardening:**
```ini
[Service]
Restart=always
RestartSec=5
WatchdogSec=60
OOMScoreAdjust=-500
LimitNOFILE=65535
MemoryMax=2G
```

### 6.2 Network Partition (Multi-Node, Phase 4+)

**Problem:** Node A can't reach Node B. Both continue operating.
Tasks might be assigned on both sides.

**Solution (Phase 4):**
- [ ] Leader election via Raft consensus (openraft crate)
- [ ] Fencing tokens on cross-node task assignment
- [ ] Split-brain detection: minimum quorum to accept writes
- [ ] Reconciliation protocol on partition heal

### 6.3 Disk Full Handling

**Problem:** Disk fills up. SQLite writes fail. Logs can't rotate.

**Solution:**
```rust
async fn check_disk_space() -> DiskStatus {
    let stat = nix::sys::statvfs::statvfs(&db_path)?;
    let free_pct = (stat.blocks_available() as f64 / stat.blocks() as f64) * 100.0;
    
    match free_pct {
        x if x < 2.0 => DiskStatus::Critical,  // Read-only mode
        x if x < 5.0 => DiskStatus::Warning,   // Stop accepting new tasks, alert
        x if x < 10.0 => DiskStatus::Low,      // Alert, aggressive cleanup
        _ => DiskStatus::Ok,
    }
}
```

**Requirements:**
- [ ] Check disk space every 60s
- [ ] < 10%: aggressive data retention cleanup
- [ ] < 5%: stop accepting new tasks, force WAL checkpoint
- [ ] < 2%: read-only mode, refuse all writes, critical alert
- [ ] Prometheus: `iad_disk_free_bytes`, `iad_disk_free_ratio`

---

## 7. Observability for Fault Detection

### 7.1 Critical Prometheus Metrics

```
# Process health
iad_daemon_fd_count                         gauge    # Current FD count
iad_daemon_rss_bytes                        gauge    # Resident memory
iad_daemon_uptime_seconds                   gauge    # Time since start
iad_daemon_recovery_count                   counter  # Crash recoveries

# Agent health  
iad_agent_status{agent, status}             gauge    # 1 if agent in this status
iad_agent_restarts_total{agent}             counter  # Restart count
iad_agent_fd_count{agent}                   gauge    # Agent FD count
iad_agent_rss_bytes{agent}                  gauge    # Agent memory
iad_agent_oom_kills_total{agent}            counter  # OOM kills
iad_agent_circuit_breaker{agent, state}     gauge    # Circuit breaker state

# Task health
iad_task_queue_depth                        gauge    # Pending tasks
iad_task_queue_capacity                     gauge    # Max queue size
iad_task_duration_seconds{agent}            histogram # Task duration
iad_task_retries_total{agent, reason}       counter  # Retry count by reason
iad_task_dead_letter_total{agent}           counter  # Dead-lettered tasks
iad_task_timeout_total{agent}               counter  # Timed-out tasks

# LLM/AI specific
iad_agent_tokens_total{agent, direction}    counter  # in/out tokens
iad_agent_cost_usd{agent}                   counter  # Accumulated cost
iad_agent_tool_calls_total{agent}           counter  # Tool invocations

# Storage
iad_db_size_bytes                           gauge    # SQLite file size
iad_wal_size_bytes                          gauge    # WAL file size
iad_disk_free_bytes                         gauge    # Available disk
```

### 7.2 Structured Log Events for Faults

```json
{"level":"error","event":"agent.oom_killed","agent":"content-writer","pid":12345,"rss_bytes":536870912,"timestamp":"2026-11-15T14:23:00Z"}
{"level":"warn","event":"task.lease_expired","task_id":"01JK...","agent":"writer","lease_ttl_s":300,"timestamp":"..."}
{"level":"error","event":"daemon.disk_critical","free_pct":1.8,"action":"read_only_mode","timestamp":"..."}
{"level":"warn","event":"agent.circuit_open","agent":"scraper","consecutive_failures":5,"cooldown_s":60,"timestamp":"..."}
{"level":"error","event":"task.cost_limit","agent":"writer","cost_usd":10.05,"limit_usd":10.00,"action":"isolate","timestamp":"..."}
```

### 7.3 Liveness vs Readiness Probes

```
Liveness (is the daemon/agent alive?):
  GET /health/live → 200 OK  |  503 Service Unavailable
  - Checks: event loop responsive, PID file valid, SQLite connectable
  
Readiness (can the daemon/agent accept work?):
  GET /health/ready → 200 OK  |  503 Service Unavailable  
  - Checks: queue not full, disk ok, not in recovery mode, not shutting down
  
Agent-level readiness:
  - Can agent accept tasks? (below max_concurrent_tasks, circuit closed, budget ok)
  - Separate from process liveness (process alive but LLM rate-limited = not ready)
```

---

## 8. Additional CLI Commands for Production Debugging

Beyond the core CLI in SPEC.md, these commands are needed for fault diagnosis:

```bash
# Process inspection
iad agent ps --tree                   # Show process tree (agent → children)
iad agent inspect <name> --resources  # FD count, RSS, CPU, cgroup stats
iad agent inspect <name> --socket     # Socket buffer stats, pending frames

# Fault management  
iad agent drain <name>                # Stop routing new tasks, wait for in-flight
iad agent isolate <name>              # Immediate: reject all tasks, keep process
iad agent heal <name>                 # Reset circuit breaker, clear failure count

# Task recovery
iad task dead-letter list             # Show dead-lettered tasks
iad task dead-letter retry <id>       # Retry specific dead-letter task
iad task dead-letter retry-all        # Retry all (with rate limit)
iad task reclaim-expired              # Force reclaim of expired leases

# System diagnostics
iad debug fd-report                   # FD usage report for daemon + all agents
iad debug db-stats                    # SQLite stats, WAL size, page count
iad debug disk-check                  # Disk space report
iad debug recovery-dry-run            # Simulate crash recovery without acting
iad debug tokio-dump                  # Dump Tokio task metrics

# Incident review
iad incident list [--agent <name>]    # List recorded incidents
iad incident show <id>                # Show incident detail + stderr tail
```

---

## 9. Schema Additions for Fault Tracking

These tables supplement the existing schema in SPEC.md:

```sql
-- Agent state extensions
ALTER TABLE agents ADD COLUMN generation_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE agents ADD COLUMN cgroup_path TEXT;
ALTER TABLE agents ADD COLUMN last_exit_code INTEGER;
ALTER TABLE agents ADD COLUMN last_exit_signal INTEGER;
ALTER TABLE agents ADD COLUMN oom_killed INTEGER DEFAULT 0;

-- Task lease tracking (extends existing tasks table)
ALTER TABLE tasks ADD COLUMN lease_token TEXT;
ALTER TABLE tasks ADD COLUMN lease_expires_at TEXT;
CREATE INDEX idx_tasks_lease ON tasks(status, lease_expires_at)
    WHERE status = 'leased' OR status = 'running';

-- Dead letter queue
CREATE TABLE dead_letter_tasks (
    id TEXT PRIMARY KEY,
    original_task_id TEXT NOT NULL REFERENCES tasks(id),
    agent_id TEXT REFERENCES agents(id),
    failure_reason TEXT NOT NULL,
    error_class TEXT NOT NULL,     -- transient, fatal, budget, timeout, loop
    original_payload TEXT NOT NULL,
    partial_result TEXT,
    retry_count INTEGER NOT NULL,
    dead_lettered_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Incident log (flight recorder)
CREATE TABLE agent_incidents (
    id TEXT PRIMARY KEY,            -- ULID
    agent_id TEXT NOT NULL REFERENCES agents(id),
    incident_type TEXT NOT NULL,    -- oom, timeout, crash, fd_exhaust, 
                                   -- circuit_open, cost_limit, tool_loop,
                                   -- context_overflow, network_error
    severity TEXT NOT NULL,         -- critical, warning, info
    exit_code INTEGER,
    signal INTEGER,
    stderr_tail TEXT,               -- Last 4KB of stderr
    context TEXT,                   -- JSON: additional diagnostic data
    resolved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_incidents_agent ON agent_incidents(agent_id, created_at);
CREATE INDEX idx_incidents_type ON agent_incidents(incident_type);

-- Health tick history (rolling window)
CREATE TABLE agent_health_ticks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    check_type TEXT NOT NULL,       -- ping, readiness, resource
    latency_ms INTEGER NOT NULL,
    status TEXT NOT NULL,           -- healthy, degraded, unhealthy, timeout
    details TEXT,                   -- JSON: resource usage snapshot
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_health_cleanup ON agent_health_ticks(recorded_at);

-- Agent usage tracking (LLM cost/token accounting)
CREATE TABLE agent_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    task_id TEXT REFERENCES tasks(id),
    tokens_in INTEGER NOT NULL DEFAULT 0,
    tokens_out INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    tool_calls INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_usage_agent_time ON agent_usage(agent_id, recorded_at);
CREATE INDEX idx_usage_rollup ON agent_usage(recorded_at); -- For hourly/daily aggregation
```

---

## 10. Configuration Additions

New sections for `iad.toml`:

```toml
[resilience]
max_concurrent_restarts = 5            # Thundering herd prevention
restart_jitter_ms = [100, 2000]        # Random jitter range
inter_batch_cooldown_s = 3             # Between restart batches

[resilience.retry]
global_budget_tokens = 100             # Token bucket size
global_budget_refill_per_sec = 10      # Refill rate
per_agent_max_backoff_s = 30           # Backoff cap
per_agent_jitter_pct = 25              # ±25% jitter

[resilience.circuit_breaker]
failure_threshold = 5                  # Consecutive failures to open
cooldown_s = 60                        # Time in open state
half_open_max_probes = 1               # Probes before closing

[resilience.queue]
global_capacity = 10000                # Max pending tasks
per_agent_in_flight = 10               # Default max concurrent per agent
overflow_policy = "reject"             # reject | persist | drop-lowest

[resilience.lease]
default_ttl_s = 300                    # 5 min default task lease
heartbeat_interval_s = 10             # Agent must heartbeat this often
grace_period_s = 30                    # Extra time before reclaim

[resilience.socket]
frame_read_timeout_s = 30
frame_write_timeout_s = 10
max_frame_size_bytes = 16777216        # 16 MB
idle_timeout_s = 300                   # 5 min no heartbeat → disconnect

[resilience.disk]
check_interval_s = 60
low_threshold_pct = 10
warning_threshold_pct = 5
critical_threshold_pct = 2

[retention]
completed_tasks_days = 30
failed_tasks_days = 90
dead_letter_days = 90
events_days = 7
health_ticks_hours = 24
incidents_days = 180
daemon_logs_days = 30
agent_logs_days = 7
cleanup_interval_s = 3600             # Run janitor every hour

[daemon]
oom_score_adj = -500
# ... existing config ...
```

---

## 11. Testing Strategies for Resilience

### 11.1 Chaos Test Suite

| Test | Method | Validates |
|------|--------|-----------|
| Agent SIGKILL | `kill -9 <pid>` during task | Zombie reap, task reclaim, restart |
| Daemon SIGKILL | `kill -9 <daemon_pid>` | Recovery sequence on restart |
| Disk full | `fallocate` to fill disk | Read-only mode, graceful degradation |
| FD exhaustion | Open 1000 files in agent | FD monitoring, agent isolation |
| Slow agent | Agent sleeps in handler | Lease timeout, task reclaim |
| Network drop | `iptables -j DROP` on LLM port | Stream timeout, retry/circuit breaker |
| OOM | Agent allocates unbounded memory | cgroup kill, OOM detection, restart |
| Orphan bomb | Agent forks grandchildren then exits | Process group kill, orphan cleanup |
| Retry storm | 20 agents fail simultaneously | Global budget limits retries/sec |
| WebSocket drop | Kill client mid-stream | Cancellation propagation, cleanup |
| Thundering herd | Restart daemon with 50 agents | Staggered restart, resource limits |
| Corrupt DB | Truncate WAL mid-write | Integrity check, recovery |
| Clock skew | Set system time forward 1h | Lease/cron/retention not broken |

### 11.2 Integration Test Framework

```rust
#[cfg(test)]
mod resilience_tests {
    use crate::test_utils::TestDaemon;
    
    #[tokio::test]
    async fn test_agent_crash_recovery() {
        let daemon = TestDaemon::spawn().await;
        let agent = daemon.register_agent("test-agent").await;
        daemon.start_agent("test-agent").await;
        
        // Kill agent process
        agent.kill(Signal::SIGKILL);
        
        // Verify recovery
        tokio::time::sleep(Duration::from_secs(5)).await;
        let status = daemon.agent_status("test-agent").await;
        assert_eq!(status, AgentStatus::Running); // Auto-restarted
        assert_eq!(daemon.agent_generation("test-agent").await, 2);
    }
    
    #[tokio::test]
    async fn test_task_lease_expiry() {
        let daemon = TestDaemon::spawn().await;
        // Register agent that hangs forever
        let agent = daemon.register_agent("hanging-agent").await;
        daemon.start_agent("hanging-agent").await;
        
        // Submit task with short lease
        let task_id = daemon.submit_task("hanging-agent", json!({}), 
            TaskConfig { lease_ttl: Duration::from_secs(5), ..default() }).await;
        
        // Wait for lease to expire
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        let task = daemon.get_task(&task_id).await;
        assert_eq!(task.status, "pending"); // Reclaimed
        assert_eq!(task.retry_count, 1);
    }
}
```

---

## 12. Implementation Priority

### Must-Have for v0.1.0 (Phase 1)
1. ✅ Zombie process reaping (§1.1)
2. ✅ Orphan process group kill (§1.2)
3. ✅ PID file flock (§1.3)
4. ✅ FD leak prevention + CLOEXEC (§1.4)
5. ✅ Daemon crash recovery sequence (§2.1)
6. ✅ Thundering herd prevention (§2.3)
7. ✅ Basic structured logging for faults (§7.2)
8. ✅ Agent generation counter (§4.2)
9. ✅ Agent incidents table (§9)

### Must-Have for v0.2.0 (Phase 2)
10. Task lease system (§3.2)
11. Backpressure on task queue (§3.3)
12. Retry storm prevention (§3.4)
13. Socket timeout + frame size cap (§3.1)
14. Dead letter queue (§9)
15. Error classification (retryable vs fatal) (§5.1)

### Must-Have for v0.3.0 (Phase 3)
16. Full Prometheus metrics (§7.1)
17. Liveness/readiness probes (§7.3)
18. WebSocket session resilience (§3.5)
19. Cost/token tracking (§5.3)
20. Data retention janitor (§4.3)
21. Disk space monitoring (§6.3)
22. Debug CLI commands (§8)

### Phase 4+ (Scale & Hardening)
23. pidfd (§1.5)
24. cgroups v2 isolation (§1.2)
25. OOM score management (§2.2)
26. Network partition handling (§6.2)
27. Chaos test suite (§11.1)
28. Hallucination loop detection (§5.2)
29. Cancellation propagation (§5.5)

---

*Document maintained by Rizoa. Updated with each resilience review.*
*Initial analysis: Gemini 3.8 Flash + Claude Sonnet deep review.*
