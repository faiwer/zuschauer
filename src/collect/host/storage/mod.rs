//! Used space of the filesystem holding each configured path. Reads the
//! filesystem's own counters, like `df`, so it's instant regardless of how
//! many files there are.
//!
//! - `run`: the collector task, one per mount
//! - `blocks`: block counters and `df`'s percentage
//! - `linux`, `macos`: per-OS `read()`

mod blocks;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod run;

pub use run::run;
