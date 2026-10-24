//! Serde types that mirror `config.jsonc` one-to-one. Everything optional
//! stays `Option` here; defaults are applied in `resolve`.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

use crate::http::Url;

use super::types::{Celsius, Dur, Percent, Severity};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawConfig {
    pub listen: SocketAddr,
    /// Names this machine in notifications.
    pub context: Option<String>,
    pub host: Option<RawHost>,
    pub docker: Option<RawDocker>,
    pub certificates: Option<RawCertificates>,
    pub services: Option<RawServices>,
    /// Keyed by app ID.
    pub apps: Option<BTreeMap<String, RawApp>>,
    /// Keyed by name.
    pub notifiers: Option<BTreeMap<String, RawNotifier>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RawNotifier {
    Ntfy(RawNtfy),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawNtfy {
    pub topic: String,
    pub token: Option<String>,
    /// The ntfy server.
    pub address: Option<Url>,
}

/// Shared by all apps' `certificates`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawCertificates {
    pub every: Option<Dur>,
    pub warn_before: Option<Dur>,
}

/// Shared by all apps' `services`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawServices {
    pub every: Option<Dur>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawApp {
    pub name: Option<String>,
    /// Keyed by name, as the Containers table shows them.
    pub containers: Option<BTreeMap<String, RawWatched>>,
    pub health_checks: Option<Vec<RawHealthCheck>>,
    /// Keyed by `host[:port]`.
    pub certificates: Option<BTreeMap<String, RawWatched>>,
    /// Keyed by systemd unit on Linux, launchd label on macOS.
    pub services: Option<BTreeMap<String, RawService>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawService {
    pub alert: Option<RawServiceAlert>,
}

/// A `RawAlert` that also fires, without waiting for `for`, when the service
/// restarts too often.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawServiceAlert {
    #[serde(rename = "for")]
    pub duration: Option<Dur>,
    pub restarts: Option<RawRestarts>,
    pub severity: Option<Severity>,
    pub id: Option<String>,
    pub depends_on: Option<Vec<String>>,
}

/// More than `above` restarts within `within`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawRestarts {
    pub above: u32,
    pub within: Dur,
}

/// Something that's either fine or not, with nothing else to configure.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawWatched {
    pub alert: Option<RawAlert>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawHealthCheck {
    pub name: Option<String>,
    pub url: Url,
    pub every: Option<Dur>,
    pub timeout: Option<Dur>,
    pub expect_code: Option<u16>,
    pub content_type: Option<String>,
    pub alert: Option<RawAlert>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawDocker {
    pub every: Option<Dur>,
    /// Docker or Podman API socket; found automatically when unset.
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawHost {
    pub cpu: Option<RawMetric>,
    pub memory: Option<RawMetric>,
    pub storage: Option<RawStorage>,
    /// Keyed by `driver/label`, as `--sensors` lists them.
    pub temperature: Option<BTreeMap<String, RawSensor>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawSensor {
    pub name: Option<String>,
    pub every: Option<Dur>,
    pub alert: Option<RawThresholdAlert<Celsius>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawStorage {
    /// Default for every mount; a mount's own `every` wins.
    pub every: Option<Dur>,
    /// Keyed by mount point.
    pub mounts: Option<BTreeMap<PathBuf, RawMetric>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawMetric {
    pub every: Option<Dur>,
    pub alert: Option<RawThresholdAlert<Percent>>,
}

/// Fires when a value stays above `above` for at least `for`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawThresholdAlert<T> {
    pub above: T,
    #[serde(rename = "for")]
    pub duration: Dur,
    /// Once fired, the alert resolves only below this. Defaults to `above`.
    pub resolve_below: Option<T>,
    pub severity: Option<Severity>,
    pub id: Option<String>,
    pub depends_on: Option<Vec<String>>,
}

/// Fires when something stays down, gone or expiring for at least `for`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawAlert {
    #[serde(rename = "for")]
    pub duration: Option<Dur>,
    pub severity: Option<Severity>,
    pub id: Option<String>,
    pub depends_on: Option<Vec<String>>,
}
