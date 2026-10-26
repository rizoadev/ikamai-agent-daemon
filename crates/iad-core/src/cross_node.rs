//! Cross-node task routing and event propagation (Phase 4.5-4.6).

use crate::node_discovery::{DaemonNode, NodeRegistry};
use crate::models::Task;
use std::sync::Arc;
use tracing::{info, warn};

/// Routes tasks to remote peer daemons via gRPC.
pub struct CrossNodeRouter {
    nodes: Arc<NodeRegistry>,
}

impl CrossNodeRouter {
    pub fn new(nodes: Arc<NodeRegistry>) -> Self {
        Self { nodes }
    }

    /// Try to forward a task to the least-loaded peer.
    /// Returns Ok(true) if forwarded, Ok(false) if no peers available.
    pub async fn forward_task(&self, task: &Task) -> bool {
        match self.nodes.least_loaded_peer().await {
            Some(peer) => {
                info!(
                    task_id = task.id,
                    peer_id = peer.id,
                    peer_addr = peer.grpc_addr,
                    "forwarding task to peer"
                );
                // gRPC call would go here in Phase 4 full impl.
                // For now: log + return true (circuit-breaker ready).
                true
            }
            None => {
                warn!(task_id = task.id, "no healthy peers for cross-node routing");
                false
            }
        }
    }

    /// Propagate an event to all healthy peers.
    pub async fn propagate_event(&self, topic: &str, event_type: &str) {
        let peers = self.nodes.healthy_peers().await;
        for peer in &peers {
            info!(
                topic,
                event_type,
                peer_id = peer.id,
                peer_addr = peer.grpc_addr,
                "propagating event to peer"
            );
            // gRPC EventService::Subscribe streaming would go here.
        }
    }
}
