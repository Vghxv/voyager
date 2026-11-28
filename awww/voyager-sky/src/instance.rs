//! One instance at a time, and a clean stop on SIGTERM / SIGINT.
//!
//! `run` and `listen` each keep a pidfile in $XDG_RUNTIME_DIR, so neither starts twice
//! and `toggle` can find the running drag mode.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

pub const RUN: &str = "voyager-sky.pid";
pub const LISTEN: &str = "voyager-sky-listen.pid";

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

/// From now on SIGTERM and SIGINT set the flag `stopping` reads, instead of killing us.
pub fn catch_stop_signals() {
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as usize);
        libc::signal(libc::SIGINT, on_signal as usize);
    }
}

pub fn stopping() -> bool {
    STOP.load(Ordering::SeqCst)
}

fn path(name: &str) -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(name)
}

/// The pid in `name`, if that process is a live voyager-sky other than this one.
pub fn running(name: &str) -> Option<i32> {
    let pid: i32 = std::fs::read_to_string(path(name))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    (comm.trim() == "voyager-sky" && pid != std::process::id() as i32).then_some(pid)
}

/// Ask the voyager-sky in `name` to stop. False if there is none.
pub fn stop(name: &str) -> bool {
    match running(name) {
        Some(pid) => {
            unsafe { libc::kill(pid, libc::SIGTERM) };
            true
        }
        None => false,
    }
}

/// Our pid, written to a pidfile; the file is removed again when this is dropped.
pub struct PidFile(PathBuf);

impl PidFile {
    pub fn claim(name: &str) -> Result<Self, String> {
        let path = path(name);
        std::fs::write(&path, std::process::id().to_string())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(PidFile(path))
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
