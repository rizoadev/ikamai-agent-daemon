//! Domain error types.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Agent '{name}' not found")]
    AgentNotFound { name: String },

    #[error("Agent '{name}' already exists")]
    AgentAlreadyExists { name: String },

    #[error("Agent '{name}' is already running (pid={pid})")]
    AgentAlreadyRunning { name: String, pid: u32 },

    #[error("Invalid manifest: {reason}")]
    InvalidManifest { reason: String },

    #[error("Task '{id}' not found")]
    TaskNotFound { id: String },

    #[error("No agent available for capabilities: {caps:?}")]
    NoCapableAgent { caps: Vec<String> },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;
