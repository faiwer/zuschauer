//! What the per-OS readers return.

use std::time::Instant;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
compile_error!("container collector supports only Linux, macOS and FreeBSD");

#[cfg(target_os = "freebsd")]
pub(super) use super::freebsd::read;

#[cfg(target_os = "macos")]
pub(super) use super::macos::read;

#[cfg(target_os = "linux")]
pub(super) async fn read(_api: Option<&super::api::Api>) -> std::io::Result<Snapshot> {
    super::linux::read()
}

/// Container ID → name, found without the API.
#[cfg(target_os = "linux")]
pub(super) use super::conmon::names as local_names;

/// Container ID → name, found without the API. macOS and FreeBSD readings
/// come named.
#[cfg(any(target_os = "macos", target_os = "freebsd"))]
pub(super) fn local_names() -> std::collections::HashMap<String, String> {
    std::collections::HashMap::new()
}

/// Everything running at one point in time.
pub(super) struct Snapshot {
    /// Cores the containers share; CPU % is relative to all of them.
    pub cores: u32,
    pub containers: Vec<Reading>,
}

pub(super) struct Reading {
    pub id: String,
    /// Known up front on macOS (from the list), for FreeBSD jails other than
    /// Podman's, and for Quadlet containers; looked up by ID otherwise.
    pub name: Option<String>,
    pub at: Instant,
    /// Cumulative CPU time, all cores.
    pub cpu_usage_usec: u64,
    /// Bytes, cache excluded. `None` when unavailable.
    pub memory: Option<u64>,
    pub paused: bool,
}
