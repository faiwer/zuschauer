//! `config.jsonc` → `&'static Config`.
//!
//! - `raw`: serde types that mirror the file
//! - `types`: value types parsed from strings (`Percent`, `Celsius`, `Dur`,
//!   `AlertRule`)
//! - `resolve`: `RawConfig` → `Config`, defaults and validation
//! - `resolved`: the final `Config`
//! - `load`: read, parse, resolve
//! - `error`: `LoadError`

mod error;
mod load;
mod raw;
mod resolve;
mod resolved;
mod types;

pub use load::load;
pub use resolved::{App, CertificatesConfig, HealthCheck, Sensor};
#[expect(unused_imports, reason = "public API, not consumed yet")]
pub use {
    error::LoadError,
    resolved::{Config, DockerConfig, HostConfig, Metric, Mount},
    types::{AlertRule, Celsius, Dur, Percent},
};
