//! PostgreSQL storage backend for IAD (Phase 4.7).
//!
//! Implements the same interface as iad-storage (SQLite) but backed by PostgreSQL.
//! Use for multi-node deployments where all daemons share state.
//!
//! Schema: see migrations/001_init.sql

use iad_core::{
    models::{Agent, AgentIncident, AgentManifest, AgentStatus, RestartPolicy},
    Error, Result,
};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::info;

/// PostgreSQL-backed storage.
#[derive(Clone)]
pub struct PgStorage {
    pool: Arc<PgPool>,
}

impl PgStorage {
    /// Connect to PostgreSQL. `url` = postgres://user:pass@host/db
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        info!("PostgreSQL storage connected");
        Ok(Self { pool: Arc::new(pool) })
    }

    pub async fn upsert_agent(&self, agent: &Agent) -> Result<()> {
        let manifest = serde_json::to_value(&agent.manifest)
            .map_err(|e| Error::Serde(e.to_string()))?;
        sqlx::query(
            "INSERT INTO agents (id, name, version, status, pid, manifest, generation_id,
             failure_count, started_at, stopped_at, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
             ON CONFLICT (id) DO UPDATE SET
               status=EXCLUDED.status, pid=EXCLUDED.pid, manifest=EXCLUDED.manifest,
               generation_id=EXCLUDED.generation_id, failure_count=EXCLUDED.failure_count,
               started_at=EXCLUDED.started_at, stopped_at=EXCLUDED.stopped_at,
               updated_at=EXCLUDED.updated_at"
        )
        .bind(&agent.id).bind(&agent.name).bind(&agent.version)
        .bind(agent.status.to_string()).bind(agent.pid.map(|p| p as i64))
        .bind(manifest).bind(agent.generation_id as i64)
        .bind(agent.failure_count as i64)
        .bind(agent.started_at).bind(agent.stopped_at)
        .bind(agent.created_at).bind(agent.updated_at)
        .execute(&*self.pool).await
        .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn list_agents(&self) -> Result<Vec<Agent>> {
        let rows = sqlx::query(
            "SELECT id,name,version,status,pid,manifest,generation_id,failure_count,
                    started_at,stopped_at,created_at,updated_at
             FROM agents ORDER BY created_at ASC"
        )
        .fetch_all(&*self.pool).await
        .map_err(|e| Error::Storage(e.to_string()))?;

        rows.iter().map(row_to_agent).collect()
    }

    pub async fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>> {
        let row = sqlx::query(
            "SELECT id,name,version,status,pid,manifest,generation_id,failure_count,
                    started_at,stopped_at,created_at,updated_at
             FROM agents WHERE name=$1"
        )
        .bind(name)
        .fetch_optional(&*self.pool).await
        .map_err(|e| Error::Storage(e.to_string()))?;
        row.as_ref().map(row_to_agent).transpose()
    }

    pub async fn delete_agent_by_name(&self, name: &str) -> Result<bool> {
        let r = sqlx::query("DELETE FROM agents WHERE name=$1")
            .bind(name)
            .execute(&*self.pool).await
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(r.rows_affected() > 0)
    }

    pub async fn insert_incident(&self, inc: &AgentIncident) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_incidents (id,agent_id,agent_name,generation_id,exit_code,signal,reason,occurred_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"
        )
        .bind(&inc.id).bind(&inc.agent_id).bind(&inc.agent_name)
        .bind(inc.generation_id as i64).bind(inc.exit_code).bind(inc.signal)
        .bind(&inc.reason).bind(inc.occurred_at)
        .execute(&*self.pool).await
        .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(())
    }
}

fn row_to_agent(row: &sqlx::postgres::PgRow) -> Result<Agent> {
    let status_str: String = row.try_get("status").map_err(|e| Error::Storage(e.to_string()))?;
    let manifest_val: serde_json::Value = row.try_get("manifest").map_err(|e| Error::Storage(e.to_string()))?;
    let manifest: AgentManifest = serde_json::from_value(manifest_val).unwrap_or_default();
    let status = match status_str.as_str() {
        "running" => AgentStatus::Running, "crashed" => AgentStatus::Crashed,
        "stopped" => AgentStatus::Stopped, "failed"  => AgentStatus::Failed,
        "starting" => AgentStatus::Starting, "stopping" => AgentStatus::Stopping,
        "disabled" => AgentStatus::Disabled, _ => AgentStatus::Registered,
    };
    Ok(Agent {
        id:            row.try_get("id").map_err(|e| Error::Storage(e.to_string()))?,
        name:          row.try_get("name").map_err(|e| Error::Storage(e.to_string()))?,
        version:       row.try_get("version").map_err(|e| Error::Storage(e.to_string()))?,
        status,
        pid:           row.try_get::<Option<i64>, _>("pid").ok().flatten().map(|p| p as u32),
        manifest,
        generation_id: row.try_get::<i64, _>("generation_id").map(|v| v as u64).unwrap_or(0),
        failure_count: row.try_get::<i64, _>("failure_count").map(|v| v as u32).unwrap_or(0),
        started_at:    row.try_get("started_at").ok(),
        stopped_at:    row.try_get("stopped_at").ok(),
        created_at:    row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
        updated_at:    row.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
    })
}
