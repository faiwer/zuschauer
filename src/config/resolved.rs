//! The final config: defaults applied, everything validated.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use crate::http::Url;

use super::types::{AlertRule, Dur};

#[derive(Debug)]
pub struct Config {
    pub listen: SocketAddr,
    pub host: HostConfig,
    /// `None` when containers aren't monitored.
    pub docker: Option<DockerConfig>,
    /// Sorted by ID.
    pub apps: Vec<App>,
}

#[derive(Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    pub health_checks: Vec<HealthCheck>,
}

#[derive(Debug)]
pub struct HealthCheck {
    /// Unique within the app.
    pub name: String,
    pub url: Url,
    pub every: Duration,
    pub timeout: Duration,
    pub expect_code: u16,
    /// Must be a substring of the response's `Content-Type`, case-insensitive.
    pub content_type: Option<String>,
}

#[derive(Debug)]
pub struct DockerConfig {
    pub every: Duration,
    /// `None` means find it automatically.
    pub socket: Option<PathBuf>,
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
        match &self.docker {
            Some(docker) => {
                write!(f, "docker: every {}, socket ", Dur::from(docker.every))?;
                match &docker.socket {
                    Some(socket) => writeln!(f, "{}", socket.display())?,
                    None => writeln!(f, "auto")?,
                }
            }
            None => writeln!(f, "docker: off")?,
        }
        for app in &self.apps {
            writeln!(f, "apps.{}: {}", app.id, app.name)?;
            for check in &app.health_checks {
                writeln!(f, "apps.{}.{}: {check}", app.id, check.name)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for HealthCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GET {} every {}, timeout {}, expect {}",
            self.url,
            Dur::from(self.every),
            Dur::from(self.timeout),
            self.expect_code,
        )?;
        if let Some(content_type) = &self.content_type {
            write!(f, ", content type {content_type}")?;
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
