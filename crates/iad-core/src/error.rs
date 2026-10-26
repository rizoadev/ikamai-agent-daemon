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

    #[error("Daemon is not running")]
    DaemonNotRunning,

    #[error("Daemon is already running (pid={pid})")]
    DaemonAlreadyRunning { pid: u32 },

    #[error("Process error: {0}")]
    Process(String),

    #[error("Serialization error: {0}")]
    Serde(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Storage(e.to_string())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::Serde(e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Config(e.to_string())
    }
}
