//! IKAMAI Agent Daemon — core library.

pub mod config;
pub mod error;
pub mod event_bus;
pub mod health;
pub mod ipc;
pub mod models;
pub mod pid;
pub mod process;
pub mod registry;
pub mod router;
pub mod scheduler;
pub mod task_queue;

pub use config::DaemonConfig;
pub use error::{Error, Result};
pub use models::{
    Agent, AgentIncident, AgentManifest, AgentStatus, RestartPolicy,
    Event, Priority, Task, TaskStatus,
};
pub use registry::AgentRegistry;

// Test modules (only compiled in test builds)
#[cfg(test)]
mod config_test;
#[cfg(test)]
mod models_test;
#[cfg(test)]
mod pid_test;
#[cfg(test)]
mod process_test;
#[cfg(test)]
mod registry_test;
