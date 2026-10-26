//! IKAMAI Agent Daemon — storage layer.
//!
//! SQLite-backed persistence for agents, tasks, events, and incidents.
//! Uses WAL mode + integrity checks per plan task 1.3.

use iad_core::{
    models::{Agent, AgentIncident, AgentManifest, AgentStatus, RestartPolicy},
    Error, Result,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

/// Thread-safe storage handle.
#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    /// Open (or create) the SQLite database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        // Create parent directories if needed.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)
            .map_err(|e| Error::Storage(e.to_string()))?;

        // WAL mode for concurrent reads + single writer.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")
            .map_err(|e| Error::Storage(e.to_string()))?;

        let storage = Self { conn: Arc::new(Mutex::new(conn)) };
        storage.run_migrations()?;
        Ok(storage)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|e| Error::Storage(e.to_string()))
    }

    // ─── Migrations ──────────────────────────────────────────────────────────

    fn run_migrations(&self) -> Result<()> {
        let conn = self.lock()?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| Error::Storage(format!("migration failed: {e}")))?;
        Ok(())
    }

    // ─── Agent Registry ──────────────────────────────────────────────────────

    /// Insert or replace an agent.
    pub fn upsert_agent(&self, agent: &Agent) -> Result<()> {
        let conn = self.lock()?;
        let manifest = serde_json::to_string(&agent.manifest)
            .map_err(|e| Error::Serde(e.to_string()))?;
        conn.execute(
            "INSERT INTO agents
             (id, name, version, status, pid, manifest, generation_id,
              failure_count, started_at, stopped_at, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(id) DO UPDATE SET
               status        = excluded.status,
               pid           = excluded.pid,
               manifest      = excluded.manifest,
               generation_id = excluded.generation_id,
               failure_count = excluded.failure_count,
               started_at    = excluded.started_at,
               stopped_at    = excluded.stopped_at,
               updated_at    = excluded.updated_at",
            params![
                agent.id,
                agent.name,
                agent.version,
                agent.status.to_string(),
                agent.pid,
                manifest,
                agent.generation_id as i64,
                agent.failure_count,
                agent.started_at.map(|t| t.to_rfc3339()),
                agent.stopped_at.map(|t| t.to_rfc3339()),
                agent.created_at.to_rfc3339(),
                agent.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }

    /// Find agent by name.
    pub fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>> {
        let conn = self.lock()?;
        let row = conn.query_row(
            "SELECT id,name,version,status,pid,manifest,generation_id,failure_count,
                    started_at,stopped_at,created_at,updated_at
             FROM agents WHERE name = ?1",
            params![name],
            row_to_agent,
        ).optional().map_err(|e| Error::Storage(e.to_string()))?;
        Ok(row)
    }

    /// Find agent by id.
    pub fn get_agent(&self, id: &str) -> Result<Option<Agent>> {
        let conn = self.lock()?;
        let row = conn.query_row(
            "SELECT id,name,version,status,pid,manifest,generation_id,failure_count,
                    started_at,stopped_at,created_at,updated_at
             FROM agents WHERE id = ?1",
            params![id],
            row_to_agent,
        ).optional().map_err(|e| Error::Storage(e.to_string()))?;
        Ok(row)
    }

    /// List all agents.
    pub fn list_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id,name,version,status,pid,manifest,generation_id,failure_count,
                    started_at,stopped_at,created_at,updated_at
             FROM agents ORDER BY created_at ASC",
        ).map_err(|e| Error::Storage(e.to_string()))?;
        let agents: std::result::Result<Vec<_>, _> = stmt
            .query_map([], row_to_agent)
            .map_err(|e| Error::Storage(e.to_string()))?
            .collect();
        agents.map_err(|e| Error::Storage(e.to_string()))
    }

    /// Delete agent by name.
    pub fn delete_agent_by_name(&self, name: &str) -> Result<bool> {
        let conn = self.lock()?;
        let n = conn.execute("DELETE FROM agents WHERE name = ?1", params![name])
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(n > 0)
    }

    // ─── Agent Incidents ─────────────────────────────────────────────────────

    pub fn insert_incident(&self, inc: &AgentIncident) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO agent_incidents
             (id, agent_id, agent_name, generation_id, exit_code, signal, reason, occurred_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                inc.id,
                inc.agent_id,
                inc.agent_name,
                inc.generation_id as i64,
                inc.exit_code,
                inc.signal,
                inc.reason,
                inc.occurred_at.to_rfc3339(),
            ],
        ).map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn list_incidents_for_agent(&self, agent_name: &str) -> Result<Vec<AgentIncident>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id,agent_id,agent_name,generation_id,exit_code,signal,reason,occurred_at
             FROM agent_incidents WHERE agent_name = ?1
             ORDER BY occurred_at DESC LIMIT 50",
        ).map_err(|e| Error::Storage(e.to_string()))?;
        let rows: std::result::Result<Vec<_>, _> = stmt
            .query_map(params![agent_name], |row| {
                Ok(AgentIncident {
                    id:           row.get(0)?,
                    agent_id:     row.get(1)?,
                    agent_name:   row.get(2)?,
                    generation_id: row.get::<_, i64>(3)? as u64,
                    exit_code:    row.get(4)?,
                    signal:       row.get(5)?,
                    reason:       row.get(6)?,
                    occurred_at:  parse_dt(row.get::<_, String>(7)?),
                })
            })
            .map_err(|e| Error::Storage(e.to_string()))?
            .collect();
        rows.map_err(|e| Error::Storage(e.to_string()))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn row_to_agent(row: &rusqlite::Row<'_>) -> rusqlite::Result<Agent> {
    let status_str: String = row.get(3)?;
    let manifest_str: String = row.get(5)?;

    let status = match status_str.as_str() {
        "registered" => AgentStatus::Registered,
        "starting"   => AgentStatus::Starting,
        "running"    => AgentStatus::Running,
        "stopping"   => AgentStatus::Stopping,
        "stopped"    => AgentStatus::Stopped,
        "failed"     => AgentStatus::Failed,
        "crashed"    => AgentStatus::Crashed,
        "disabled"   => AgentStatus::Disabled,
        _            => AgentStatus::Registered,
    };

    let manifest: AgentManifest = serde_json::from_str(&manifest_str)
        .unwrap_or_else(|_| AgentManifest {
            name: "unknown".into(),
            version: "0.0.0".into(),
            runtime: "unknown".into(),
            entrypoint: "".into(),
            args: vec![],
            capabilities: vec![],
            env: Default::default(),
            restart: RestartPolicy::default(),
            max_restarts: 5,
        });

    Ok(Agent {
        id:            row.get(0)?,
        name:          row.get(1)?,
        version:       row.get(2)?,
        status,
        pid:           row.get(4)?,
        manifest,
        generation_id: row.get::<_, i64>(6)? as u64,
        failure_count: row.get(7)?,
        started_at:    row.get::<_, Option<String>>(8)?.map(parse_dt),
        stopped_at:    row.get::<_, Option<String>>(9)?.map(parse_dt),
        created_at:    parse_dt(row.get::<_, String>(10)?),
        updated_at:    parse_dt(row.get::<_, String>(11)?),
    })
}

fn parse_dt(s: String) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

// ─── Schema ──────────────────────────────────────────────────────────────────

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS agents (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    version       TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'registered',
    pid           INTEGER,
    manifest      TEXT NOT NULL,
    generation_id INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    started_at    TEXT,
    stopped_at    TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_incidents (
    id            TEXT PRIMARY KEY,
    agent_id      TEXT NOT NULL,
    agent_name    TEXT NOT NULL,
    generation_id INTEGER NOT NULL,
    exit_code     INTEGER,
    signal        INTEGER,
    reason        TEXT NOT NULL,
    occurred_at   TEXT NOT NULL,
    FOREIGN KEY(agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_incidents_agent ON agent_incidents(agent_name, occurred_at DESC);

CREATE TABLE IF NOT EXISTS tasks (
    id             TEXT PRIMARY KEY,
    title          TEXT,
    payload        TEXT NOT NULL,
    capabilities   TEXT NOT NULL,
    priority       INTEGER NOT NULL DEFAULT 3,
    status         TEXT NOT NULL DEFAULT 'pending',
    assigned_agent TEXT,
    result         TEXT,
    error          TEXT,
    retry_count    INTEGER NOT NULL DEFAULT 0,
    max_retries    INTEGER NOT NULL DEFAULT 3,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status, priority, created_at);

CREATE TABLE IF NOT EXISTS events (
    id         TEXT PRIMARY KEY,
    topic      TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload    TEXT NOT NULL,
    source     TEXT NOT NULL,
    sequence   INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_topic ON events(topic, sequence);
"#;
