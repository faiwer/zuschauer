//! The final config: defaults applied, everything validated.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use super::types::{AlertRule, Dur};

#[derive(Debug)]
pub struct Config {
    pub listen: SocketAddr,
    pub host: HostConfig,
}

#[derive(Debug)]
pub struct HostConfig {
    pub cpu: Metric,
    pub memory: Metric,
    pub storage: Vec<Mount>,
}

#[derive(Debug)]
pub struct Metric {
    pub every: Duration,
    pub alert: Option<AlertRule>,
}

#[derive(Debug)]
pub struct Mount {
    pub path: PathBuf,
    pub metric: Metric,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "listen: {}", self.listen)?;
        writeln!(f, "host.cpu: {}", self.host.cpu)?;
        writeln!(f, "host.memory: {}", self.host.memory)?;
        for mount in &self.host.storage {
            writeln!(f, "host.storage {}: {}", mount.path.display(), mount.metric)?;
        }
        Ok(())
    }
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "every {}", Dur::from(self.every))?;
        match &self.alert {
            Some(alert) => write!(f, ", alert above {} for {}", alert.above, alert.duration),
            None => write!(f, ", no alert"),
        }
    }
}
