//! Temperature sensors, read from Linux's hwmon interface or FreeBSD's
//! sysctls. Sensors are found once at startup, then each configured one is
//! read on its own interval. macOS has no hwmon, so no sensor is found there.
//!
//! - `hwmon`, `freebsd`: finding sensors and reading them
//! - `spawn`: matches configured sensors with found ones, one task each
//! - `run`: the collector task
//! - `list`: the `--sensors` output

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(not(target_os = "freebsd"))]
mod hwmon;
mod list;
mod run;
mod spawn;

pub use list::list;
pub use spawn::spawn;
