//! Host metrics: CPU, memory, storage and temperature.
//!
//! - `spawn`: starts one task per metric (and per mount and sensor)
//! - `cpu`, `memory`, `storage`, `temperature`: the collectors
//! - `mach`: macOS kernel API bits shared by the collectors

mod cpu;
#[cfg(target_os = "macos")]
mod mach;
mod memory;
mod spawn;
mod storage;
mod temperature;

pub use spawn::spawn;
pub use temperature::list as list_sensors;
