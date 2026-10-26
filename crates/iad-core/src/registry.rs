//! Agent registry — in-memory + persisted to storage.

use crate::{
    models::{Agent, AgentManifest, AgentStatus},
    Error, Result,
};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use ulid::Ulid;

/// In-memory agent registry backed by storage.
#[derive(Clone)]
pub struct AgentRegistry {
    agents: Arc<RwLock<Vec<Agent>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Load agents from persisted list.
    pub async fn load_all(&self, agents: Vec<Agent>) {
        let mut lock = self.agents.write().await;
        *lock = agents;
    }

    /// Register a new agent from manifest.
    pub async fn register(&self, manifest: AgentManifest) -> Result<Agent> {
        let mut lock = self.agents.write().await;
        if lock.iter().any(|a| a.name == manifest.name) {
            return Err(Error::AgentAlreadyExists { name: manifest.name });
        }
        let now = Utc::now();
        let agent = Agent {
            id:            Ulid::new().to_string(),
            name:          manifest.name.clone(),
            version:       manifest.version.clone(),
            status:        AgentStatus::Registered,
            pid:           None,
            manifest,
            generation_id: 0,
            failure_count: 0,
            started_at:    None,
            stopped_at:    None,
            created_at:    now,
            updated_at:    now,
        };
        info!(agent = agent.name, id = agent.id, "agent registered");
        lock.push(agent.clone());
        Ok(agent)
    }

    /// Remove an agent by name.
    pub async fn deregister(&self, name: &str) -> Result<Agent> {
        let mut lock = self.agents.write().await;
        let pos = lock.iter().position(|a| a.name == name)
            .ok_or_else(|| Error::AgentNotFound { name: name.into() })?;
        Ok(lock.remove(pos))
    }

    /// Get agent by name (clone).
    pub async fn get(&self, name: &str) -> Result<Agent> {
        let lock = self.agents.read().await;
        lock.iter()
            .find(|a| a.name == name)
            .cloned()
            .ok_or_else(|| Error::AgentNotFound { name: name.into() })
    }

    /// List all agents.
    pub async fn list(&self) -> Vec<Agent> {
        self.agents.read().await.clone()
    }

    /// Update agent in-place.
    pub async fn update<F>(&self, name: &str, f: F) -> Result<Agent>
    where
        F: FnOnce(&mut Agent),
    {
        let mut lock = self.agents.write().await;
        let agent = lock.iter_mut()
            .find(|a| a.name == name)
            .ok_or_else(|| Error::AgentNotFound { name: name.into() })?;
        f(agent);
        agent.updated_at = Utc::now();
        Ok(agent.clone())
    }

    /// Find agents capable of handling given capabilities.
    pub async fn find_capable(&self, caps: &[String]) -> Vec<Agent> {
        let lock = self.agents.read().await;
        lock.iter()
            .filter(|a| {
                a.status == AgentStatus::Running
                    && caps.iter().all(|c| a.manifest.capabilities.contains(c))
            })
            .cloned()
            .collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self { Self::new() }
}
