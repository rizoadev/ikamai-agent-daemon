//! PID file management with flock to prevent TOCTOU races (plan D6).

use crate::{Error, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;

/// Acquire an exclusive flock on the PID file and write our PID.
/// Returns the opened file (keep it alive to maintain the lock).
pub fn acquire_pid_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    // Attempt non-blocking exclusive flock.
    let fd = file.as_raw_fd();
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        // Another process holds the lock — read its PID.
        let mut f = File::open(path)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;
        let pid: u32 = buf.trim().parse().unwrap_or(0);
        return Err(Error::DaemonAlreadyRunning { pid });
    }

    // Write our PID.
    let pid = std::process::id();
    let mut f = &file;
    f.write_all(format!("{}\n", pid).as_bytes())?;

    Ok(file)
}

/// Read PID from file. Returns None if file doesn't exist or is invalid.
pub fn read_pid(path: &Path) -> Option<u32> {
    let mut f = File::open(path).ok()?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).ok()?;
    buf.trim().parse().ok()
}

/// Remove PID file.
pub fn remove_pid_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Check if process with given PID is alive.
pub fn pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
