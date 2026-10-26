//! Turns `RawConfig` into the final `Config`: applies defaults and runs
//! checks that serde can't express. Collects every problem instead of
//! stopping at the first one.
//!
//! - `root`: `resolve()` and the small top-level sections
//! - `host`: CPU, memory, storage and temperature
//! - `apps`: apps with their health checks, containers and certificates
//! - `services`: apps' services and their shared settings
//! - `commands`: apps' commands, the programs zuschauer runs
//! - `jobs`: apps' scheduled jobs
//! - `alerts`: alert IDs, labels and dependencies
//! - `notifiers`: notifiers
//! - `heartbeat`: the request that tells an outside service zuschauer runs
//! - `template`: `%{variable}` placeholders in the heartbeat
//! - `every`: the `every` interval every section has

mod alerts;
mod apps;
mod commands;
mod every;
mod heartbeat;
mod host;
mod jobs;
mod notifiers;
mod root;
mod services;
mod template;

pub use root::resolve;
