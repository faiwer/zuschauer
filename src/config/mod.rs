//! `config.jsonc` → `&'static Config`.
//!
//! - `raw`: serde types that mirror the file
//! - `types`: value types parsed from strings (`Percent`, `Celsius`, `Dur`,
//!   `Severity`)
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
    Alert, App, AppCertificate, CertificatesConfig, Config, DockerConfig, HealthCheck, HostConfig,
    Metric, Mount, Notifier, NotifierKind, Ntfy, Sensor,
};
pub use types::{Dur, Severity};
#[expect(unused_imports, reason = "public API, not consumed yet")]
pub use {
    error::LoadError,
    resolved::{AppContainer, Threshold},
    types::{Celsius, Percent},
};
