#[cfg(test)]
mod tests {
    use crate::pid::{acquire_pid_file, pid_alive, read_pid, remove_pid_file};
    use tempfile::TempDir;

    #[test]
    fn test_acquire_and_read_pid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.pid");
        let _f = acquire_pid_file(&path).expect("acquire should succeed");
        let pid = read_pid(&path).expect("should read pid");
        assert_eq!(pid, std::process::id());
    }

    #[test]
    fn test_double_acquire_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test2.pid");
        let _f = acquire_pid_file(&path).expect("first acquire");
        let result = acquire_pid_file(&path);
        assert!(result.is_err(), "second acquire should fail");
    }

    #[test]
    fn test_pid_alive_self() {
        assert!(pid_alive(std::process::id()), "self must be alive");
    }

    #[test]
    fn test_pid_alive_dead() {
        assert!(!pid_alive(99999999));
    }

    #[test]
    fn test_remove_pid_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test3.pid");
        std::fs::write(&path, "12345
").unwrap();
        remove_pid_file(&path);
        assert!(!path.exists());
    }
}
