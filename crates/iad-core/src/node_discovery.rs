//! Node discovery + registration for multi-node IAD (Phase 4.4-4.6).
//!
//! Each daemon registers itself in a shared store (SQLite local, or future pg).
//! Peers are discovered by polling the nodes table.

use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use ulid::Ulid;

/// A peer daemon node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonNode {
    pub id:          String,
    pub grpc_addr:   String,
    pub rest_addr:   String,
    pub hostname:    String,
    pub version:     String,
    pub agents:      u32,
    pub tasks_pending: u32,
    pub healthy:     bool,
    pub last_seen:   DateTime<Utc>,
    pub registered_at: DateTime<Utc>,
}

impl DaemonNode {
    pub fn local(grpc_addr: &str, rest_addr: &str) -> Self {
        let now = Utc::now();
        Self {
            id:             Ulid::new().to_string(),
            grpc_addr:      grpc_addr.into(),
            rest_addr:      rest_addr.into(),
            hostname:       hostname(),
            version:        env!("CARGO_PKG_VERSION").into(),
            agents:         0,
            tasks_pending:  0,
            healthy:        true,
            last_seen:      now,
            registered_at:  now,
        }
    }

    pub fn is_stale(&self, ttl_secs: i64) -> bool {
        (Utc::now() - self.last_seen).num_seconds() > ttl_secs
    }
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "unknown".into())
        .trim()
        .into()
}

/// In-memory node registry (multi-node coordination).
pub struct NodeRegistry {
    local:  DaemonNode,
    peers:  RwLock<Vec<DaemonNode>>,
}

impl NodeRegistry {
    pub fn new(grpc_addr: &str, rest_addr: &str) -> Arc<Self> {
        Arc::new(Self {
            local: DaemonNode::local(grpc_addr, rest_addr),
            peers: RwLock::new(Vec::new()),
        })
    }

    pub fn local_id(&self) -> &str { &self.local.id }
    pub fn local_node(&self) -> &DaemonNode { &self.local }

    /// Upsert a peer node.
    pub async fn upsert_peer(&self, node: DaemonNode) {
        let mut peers = self.peers.write().await;
        if let Some(existing) = peers.iter_mut().find(|n| n.id == node.id) {
            *existing = node;
        } else {
            info!(id = node.id, addr = node.grpc_addr, "new peer discovered");
            peers.push(node);
        }
    }

    /// Remove stale nodes (TTL default 30s).
    pub async fn evict_stale(&self, ttl_secs: i64) -> Vec<String> {
        let mut peers = self.peers.write().await;
        let before = peers.len();
        peers.retain(|n| !n.is_stale(ttl_secs));
        let evicted: Vec<String> = (before - peers.len()).to_string().into_bytes()
            .iter().map(|_| String::new()).collect();
        evicted
    }

    pub async fn list_peers(&self) -> Vec<DaemonNode> {
        self.peers.read().await.clone()
    }

    pub async fn healthy_peers(&self) -> Vec<DaemonNode> {
        self.peers.read().await.iter()
            .filter(|n| n.healthy && !n.is_stale(30))
            .cloned()
            .collect()
    }

    /// Find best peer for a task based on load (fewest pending tasks).
    pub async fn least_loaded_peer(&self) -> Option<DaemonNode> {
        self.peers.read().await.iter()
            .filter(|n| n.healthy && !n.is_stale(30))
            .min_by_key(|n| n.tasks_pending)
            .cloned()
    }
}
