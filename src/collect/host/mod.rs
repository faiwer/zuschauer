//! Host metrics: CPU, memory and storage.
//!
//! - `spawn`: starts one task per metric (and per mount)
//! - `cpu`, `memory`, `storage`: the collectors
//! - `mach`: macOS kernel API bits shared by the collectors

mod cpu;
#[cfg(target_os = "macos")]
mod mach;
mod memory;
mod spawn;
mod storage;

pub use spawn::spawn;
