//! Daemon configuration.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    pub daemon: DaemonSection,
    pub storage: StorageSection,
    pub api: ApiSection,
    pub health: HealthSection,
}

#[derive(Debug, Deserialize)]
pub struct DaemonSection {
    pub pid_file: String,
    pub socket: String,
    pub log_level: String,
    pub log_format: String,
    pub workers: usize,
}

#[derive(Debug, Deserialize)]
pub struct StorageSection {
    pub backend: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiSection {
    pub rest_enabled: bool,
    pub rest_bind: String,
    pub grpc_enabled: bool,
    pub grpc_bind: String,
    pub websocket_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct HealthSection {
    pub default_check_interval: String,
    pub default_timeout: String,
    pub default_max_failures: u32,
}
