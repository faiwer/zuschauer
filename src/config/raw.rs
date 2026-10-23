//! Serde types that mirror `config.jsonc` one-to-one. Everything optional
//! stays `Option` here; defaults are applied in `resolve`.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

use super::types::{AlertRule, Dur};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawConfig {
    pub listen: SocketAddr,
    pub host: Option<RawHost>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawHost {
    pub cpu: Option<RawMetric>,
    pub memory: Option<RawMetric>,
    /// Keyed by mount point.
    pub storage: Option<BTreeMap<PathBuf, RawMetric>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawMetric {
    pub every: Option<Dur>,
    pub alert: Option<AlertRule>,
}
