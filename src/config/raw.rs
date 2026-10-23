//! Serde types that mirror `config.jsonc` one-to-one. Everything optional
//! stays `Option` here; defaults are applied in `resolve`.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

use crate::http::Url;

use super::types::{AlertRule, Dur};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawConfig {
    pub listen: SocketAddr,
    pub host: Option<RawHost>,
    pub docker: Option<RawDocker>,
    /// Keyed by app ID.
    pub apps: Option<BTreeMap<String, RawApp>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawApp {
    pub name: Option<String>,
    pub health_checks: Option<Vec<RawHealthCheck>>,
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
    pub alert: Option<AlertRule>,
}
