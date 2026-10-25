//! Containers (Docker or Podman): running/paused/gone, CPU and memory.
//!
//! - `run`: the collector task
//! - `tracker`: names, "gone" detection, writing to the store
//! - `reading`: one snapshot of running containers, per-OS `read()`
//! - `linux`: cgroup scan; `macos`: Docker API list + stats
//! - `cgroup`: pure cgroup parsers
//! - `conmon`: Podman container names from `conmon` command lines
//! - `cpu`: CPU % between two readings
//! - `api`: HTTP over the Docker/Podman unix socket
//! - `apps`: alerts of the containers apps list

mod api;
mod apps;
#[cfg(any(target_os = "linux", test))]
mod cgroup;
#[cfg(any(target_os = "linux", test))]
mod conmon;
mod cpu;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod reading;
mod run;
mod tracker;

pub use run::run;
