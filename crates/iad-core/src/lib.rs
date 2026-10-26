//! IKAMAI Agent Daemon — core library.

pub mod config;
pub mod error;
pub mod health;
pub mod models;
pub mod pid;
pub mod process;
pub mod registry;

pub use config::DaemonConfig;
pub use error::{Error, Result};
pub use models::{
    Agent, AgentIncident, AgentManifest, AgentStatus, RestartPolicy,
    Event, Priority, Task, TaskStatus,
};
pub use registry::AgentRegistry;
