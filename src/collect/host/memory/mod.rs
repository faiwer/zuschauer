//! Real memory usage: what programs hold, excluding cache the kernel can
//! drop at any time.
//!
//! - `run`: the collector task
//! - `usage`: used/total bytes and the percentage
//! - `linux`, `macos`, `freebsd`: per-OS `read()`

#[cfg(any(target_os = "freebsd", test))]
mod freebsd;
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod run;
mod usage;

pub use run::run;
