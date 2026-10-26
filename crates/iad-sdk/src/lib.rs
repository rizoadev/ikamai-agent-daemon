//! IKAMAI Agent Daemon — Rust SDK.
//!
//! Use this crate to build agents that integrate with the IAD daemon.

pub struct AgentClient;

impl AgentClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}
