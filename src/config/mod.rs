//! `config.jsonc` → `&'static Config`.
//!
//! - `raw`: serde types that mirror the file
//! - `types`: value types parsed from strings (`Percent`, `Celsius`, `Dur`,
//!   `Severity`, `OutputFrom`)
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
pub use resolved::{
    Alert, App, AppCertificate, AppCommand, AppJob, AppService, CertificatesConfig, Config,
    DockerConfig, HealthCheck, Heartbeat, HostConfig, Metric, Mount, Notifier, NotifierKind, Ntfy,
    OutputSlice, RestartLimit, Sensor, ServicesConfig,
};
pub use types::{Dur, OutputFrom, Severity};
#[expect(unused_imports, reason = "public API, not consumed yet")]
pub use {
    error::LoadError,
    resolved::{AppContainer, Threshold},
    types::{Celsius, Percent},
};
