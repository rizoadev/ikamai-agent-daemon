//! cgroups v2 resource limits per agent process (Phase 4.13).
//!
//! Creates a cgroup at /sys/fs/cgroup/iad/<agent_name>/
//! Applies CPU, memory, and I/O limits.

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Resource limits for an agent.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// CPU quota in microseconds per period (e.g. 50000 = 50% of 1 core).
    pub cpu_quota_us:  Option<u64>,
    /// CPU period in microseconds (default 100000 = 100ms).
    pub cpu_period_us: u64,
    /// Memory limit in bytes (e.g. 512 * 1024 * 1024 = 512 MiB).
    pub memory_max:    Option<u64>,
    /// Memory + swap limit in bytes.
    pub memory_swap:   Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_quota_us:  None,
            cpu_period_us: 100_000,
            memory_max:    None,
            memory_swap:   None,
        }
    }
}

/// cgroup v2 manager for a single agent.
pub struct CgroupManager {
    path: PathBuf,
}

impl CgroupManager {
    const CGROUP_ROOT: &'static str = "/sys/fs/cgroup/iad";

    pub fn new(agent_name: &str) -> Self {
        Self {
            path: PathBuf::from(Self::CGROUP_ROOT).join(agent_name),
        }
    }

    /// Create cgroup directory.
    pub fn create(&self) -> Result<()> {
        if !self.path.exists() {
            std::fs::create_dir_all(&self.path)?;
            info!(path = %self.path.display(), "cgroup created");
        }
        Ok(())
    }

    /// Apply resource limits.
    pub fn apply(&self, limits: &ResourceLimits) -> Result<()> {
        self.create()?;

        // CPU quota
        if let Some(quota) = limits.cpu_quota_us {
            let val = format!("{} {}", quota, limits.cpu_period_us);
            self.write("cpu.max", &val)?;
            debug!(cgroup = %self.path.display(), cpu_max = val, "cpu limit set");
        }

        // Memory limit
        if let Some(mem) = limits.memory_max {
            self.write("memory.max", &mem.to_string())?;
            debug!(cgroup = %self.path.display(), memory_max = mem, "memory limit set");
        }

        // Memory swap
        if let Some(swap) = limits.memory_swap {
            self.write("memory.swap.max", &swap.to_string())?;
        }

        Ok(())
    }

    /// Add a PID to the cgroup.
    pub fn add_pid(&self, pid: u32) -> Result<()> {
        self.write("cgroup.procs", &pid.to_string())?;
        info!(cgroup = %self.path.display(), pid, "pid added to cgroup");
        Ok(())
    }

    /// Remove cgroup (after agent stops).
    pub fn destroy(&self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_dir(&self.path) {
                warn!(path = %self.path.display(), "cgroup remove failed: {e}");
            }
        }
    }

    fn write(&self, file: &str, value: &str) -> Result<()> {
        let p = self.path.join(file);
        std::fs::write(&p, value).map_err(|e| {
            // Non-fatal if cgroups not available (e.g. in test env).
            warn!(path = %p.display(), "cgroup write failed: {e} (cgroups v2 available?)");
            Error::Io(e)
        })
    }
}

/// Check if cgroups v2 is available on this system.
pub fn is_cgroupv2_available() -> bool {
    Path::new("/sys/fs/cgroup/cgroup.controllers").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_limits_default() {
        let r = ResourceLimits::default();
        assert!(r.cpu_quota_us.is_none());
        assert_eq!(r.cpu_period_us, 100_000);
        assert!(r.memory_max.is_none());
    }

    #[test]
    fn cgroupv2_check_no_panic() {
        // Just verify it doesn't panic
        let _ = is_cgroupv2_available();
    }
}
