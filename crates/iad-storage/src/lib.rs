//! IKAMAI Agent Daemon — storage layer.
//!
//! Provides SQLite-backed persistence for agents, tasks, events, and schedules.

pub struct Storage;

impl Storage {
    pub fn new(_path: &str) -> iad_core::Result<Self> {
        // TODO: Initialize SQLite, run migrations
        Ok(Self)
    }
}
