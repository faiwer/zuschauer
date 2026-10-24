//! Turns `RawConfig` into the final `Config`: applies defaults and runs
//! checks that serde can't express. Collects every problem instead of
//! stopping at the first one.
//!
//! - `root`: `resolve()` and the small top-level sections
//! - `host`: CPU, memory, storage and temperature
//! - `apps`: apps with their health checks, containers and certificates
//! - `services`: apps' services and their shared settings
//! - `alerts`: alert IDs, labels and dependencies
//! - `notifiers`: notifiers
//! - `every`: the `every` interval every section has

mod alerts;
mod apps;
mod every;
mod host;
mod notifiers;
mod root;
mod services;

pub use root::resolve;
