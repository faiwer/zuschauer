//! Apps: HTTP(S) health checks, one task per check.
//!
//! - `run`: the collector task for one check
//! - `verdict`: whether a response counts as healthy

mod run;
mod verdict;

pub use run::run;
