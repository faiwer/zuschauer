//! Temperature sensors, read from Linux's hwmon interface. Sensors are found
//! once at startup, then each configured one is read on its own interval.
//! Other OSes have no hwmon, so no sensor is found there.
//!
//! - `hwmon`: finding sensors and reading them
//! - `spawn`: matches configured sensors with found ones, one task each
//! - `run`: the collector task
//! - `list`: the `--sensors` output

mod hwmon;
mod list;
mod run;
mod spawn;

pub use list::list;
pub use spawn::spawn;
