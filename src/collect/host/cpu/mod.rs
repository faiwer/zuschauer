//! Total CPU usage across all cores, as the average over each `every`
//! interval.
//!
//! - `run`: the collector task
//! - `ticks`: cumulative tick counters and the busy share between two readings
//! - `linux`, `macos`: per-OS `read()`

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod run;
mod ticks;

pub use run::run;
