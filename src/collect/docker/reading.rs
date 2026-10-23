//! What the per-OS readers return.

use std::time::Instant;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("container collector supports only Linux and macOS");

#[cfg(target_os = "macos")]
pub(super) use super::macos::read;

#[cfg(target_os = "linux")]
pub(super) async fn read(_api: Option<&super::api::Api>) -> std::io::Result<Snapshot> {
    super::linux::read()
}

/// Everything running at one point in time.
pub(super) struct Snapshot {
    /// Cores the containers share; CPU % is relative to all of them.
    pub cores: u32,
    pub containers: Vec<Reading>,
}

pub(super) struct Reading {
    pub id: String,
    /// Known up front on macOS (from the list); looked up by ID on Linux.
    pub name: Option<String>,
    pub at: Instant,
    /// Cumulative CPU time, all cores.
    pub cpu_usage_usec: u64,
    /// Bytes, cache excluded. `None` when unavailable.
    pub memory: Option<u64>,
    pub paused: bool,
}
