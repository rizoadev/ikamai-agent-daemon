#[cfg(test)]
mod tests {
    use crate::config::DaemonConfig;
    use tempfile::TempDir;

    #[test]
    fn default_config_is_valid() {
        let cfg = DaemonConfig::default();
        assert_eq!(cfg.daemon.workers, 4);
        assert_eq!(cfg.api.rest_bind, "127.0.0.1:7700");
        assert_eq!(cfg.health.max_failures, 3);
    }

    #[test]
    fn load_from_missing_file_gives_default() {
        let cfg = DaemonConfig::load(std::path::Path::new("/nonexistent/iad.toml")).unwrap();
        assert_eq!(cfg.daemon.log_level, "info");
    }

    #[test]
    fn load_from_toml_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("iad.toml");
        std::fs::write(&path, r#"
[daemon]
pid_file   = "/tmp/iad.pid"
socket     = "/tmp/iad.sock"
log_level  = "debug"
log_format = "json"
workers    = 8

[storage]
backend = "sqlite"
path    = "/tmp/iad.db"

[api]
rest_enabled      = true
rest_bind         = "0.0.0.0:8080"
grpc_enabled      = false
grpc_bind         = "127.0.0.1:7701"
websocket_enabled = false

[health]
check_interval_secs = 5
timeout_secs        = 3
max_failures        = 5
"#).unwrap();
        let cfg = DaemonConfig::load(&path).unwrap();
        assert_eq!(cfg.daemon.log_level, "debug");
        assert_eq!(cfg.daemon.workers, 8);
        assert_eq!(cfg.api.rest_bind, "0.0.0.0:8080");
        assert_eq!(cfg.health.max_failures, 5);
    }
}
