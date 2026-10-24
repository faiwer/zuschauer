//! Services: systemd units on Linux, launchd jobs on macOS. One task polls
//! all of them.
//!
//! - `run`: the task
//! - `unit`: what the service manager reports about one service
//! - `restarts`: a sliding window over a restart counter
//! - `systemd`: `systemctl show`, Linux
//! - `launchd`: `launchctl print`, macOS
//! - `command`: runs those with a timeout

mod command;
#[cfg(any(target_os = "macos", test))]
mod launchd;
mod restarts;
mod run;
#[cfg(any(target_os = "linux", test))]
mod systemd;
mod unit;

pub use run::run;
