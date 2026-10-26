//! The final config: defaults applied, everything validated.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use crate::cron::{Schedule, format_local};
use crate::http::{HostPort, Url};
use crate::model::now;

use super::types::{Dur, Severity};

#[derive(Debug)]
pub struct Config {
    pub listen: SocketAddr,
    /// Names this machine in notifications. Defaults to the hostname.
    pub context: String,
    pub host: HostConfig,
    /// `None` when containers aren't monitored.
    pub docker: Option<DockerConfig>,
    pub certificates: CertificatesConfig,
    pub services: ServicesConfig,
    /// Sorted by ID.
    pub apps: Vec<App>,
    /// Sorted by name.
    pub notifiers: Vec<Notifier>,
    pub heartbeat: Option<Heartbeat>,
}

/// The request zuschauer sends every `every`, its templates filled in.
#[derive(Debug)]
pub struct Heartbeat {
    pub url: Url,
    /// Upper case.
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub every: Duration,
}

#[derive(Debug)]
pub struct Notifier {
    pub name: String,
    pub kind: NotifierKind,
}

#[derive(Debug)]
pub enum NotifierKind {
    Ntfy(Ntfy),
}

#[derive(Debug)]
pub struct Ntfy {
    /// The server; messages are published to its root. `None` for the
    /// notifier's default, the public ntfy.sh.
    pub address: Option<Url>,
    pub topic: String,
    pub token: Option<String>,
}

#[derive(Debug)]
pub struct CertificatesConfig {
    pub every: Duration,
    /// A certificate expiring sooner than this counts as expiring.
    pub warn_before: Duration,
}

#[derive(Debug)]
pub struct ServicesConfig {
    pub every: Duration,
}

#[derive(Debug)]
pub struct App {
    pub id: String,
    pub name: String,
    /// Their data comes from the docker collector.
    pub containers: Vec<AppContainer>,
    pub health_checks: Vec<HealthCheck>,
    pub certificates: Vec<AppCertificate>,
    pub services: Vec<AppService>,
    pub jobs: Vec<AppJob>,
}

#[derive(Debug)]
pub struct AppJob {
    /// Letters, digits, `.`, `_` and `-`, so it fits in a URL as it is.
    pub name: String,
    /// Reports must carry it as `Authorization: Bearer <token>`.
    pub token: String,
    pub schedule: Schedule,
    /// A run that hasn't finished after this is hung. `None`: it must
    /// finish before the next run is due.
    pub max_duration: Option<Duration>,
    pub alert: Option<Alert>,
}

#[derive(Debug)]
pub struct AppService {
    /// A systemd unit on Linux (`.service` optional), a launchd label on
    /// macOS (`system/` or `gui/<uid>/` optional).
    pub name: String,
    pub alert: Option<Alert>,
}

#[derive(Debug, Clone, Copy)]
pub struct RestartLimit {
    pub above: u32,
    pub within: Duration,
}

#[derive(Debug)]
pub struct AppContainer {
    pub name: String,
    pub alert: Option<Alert>,
}

#[derive(Debug)]
pub struct AppCertificate {
    pub target: HostPort,
    pub alert: Option<Alert>,
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
    pub alert: Option<Alert>,
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
    /// Sorted by ID.
    pub temperature: Vec<Sensor>,
}

#[derive(Debug)]
pub struct Sensor {
    /// `driver/label`, matched against what the OS reports.
    pub id: String,
    pub name: String,
    pub every: Duration,
    pub alert: Option<Alert>,
}

#[derive(Debug)]
pub struct Metric {
    pub every: Duration,
    pub alert: Option<Alert>,
}

#[derive(Debug)]
pub struct Mount {
    pub path: PathBuf,
    pub metric: Metric,
}

#[derive(Debug)]
pub struct Alert {
    /// Unique. Derived from where the alert is unless the config sets it.
    pub id: String,
    /// What notifications call the monitored thing, e.g. "CPU".
    pub label: String,
    pub severity: Severity,
    /// How long a problem must last before the alert fires.
    pub after: Duration,
    /// IDs of every alert this one depends on, directly or not. While any of
    /// them sees a problem, this one doesn't fire.
    pub parents: Vec<String>,
    /// `None` for alerts on something down, gone or expiring.
    pub threshold: Option<Threshold>,
    /// Services only: too many restarts fire it without waiting for `after`.
    pub restarts: Option<RestartLimit>,
}

#[derive(Debug)]
pub struct Threshold {
    pub above: f32,
    pub resolve_below: Option<f32>,
    pub unit: &'static str,
}

impl Threshold {
    /// Whether `value` is a problem. Once it is (`active`), it stays one
    /// until the value drops below `resolve_below`.
    pub fn exceeded(&self, value: f32, active: bool) -> bool {
        match self.resolve_below {
            Some(below) if active => value >= below,
            _ => value > self.above,
        }
    }
}

impl Config {
    /// Reads everything every `every` instead, for `--state --every`.
    pub fn set_every(&mut self, every: Duration) {
        let host = &mut self.host;
        host.cpu.every = every;
        host.memory.every = every;
        for mount in &mut host.storage {
            mount.metric.every = every;
        }
        for sensor in &mut host.temperature {
            sensor.every = every;
        }
        if let Some(docker) = &mut self.docker {
            docker.every = every;
        }
        self.certificates.every = every;
        self.services.every = every;
        for check in self.apps.iter_mut().flat_map(|app| &mut app.health_checks) {
            check.every = every;
            check.timeout = check.timeout.min(every);
        }
    }

    pub fn alerts(&self) -> impl Iterator<Item = &Alert> {
        let host = &self.host;
        let apps = self.apps.iter().flat_map(|app| {
            let checks = app.health_checks.iter().map(|c| &c.alert);
            let containers = app.containers.iter().map(|c| &c.alert);
            let certificates = app.certificates.iter().map(|c| &c.alert);
            let services = app.services.iter().map(|s| &s.alert);
            let jobs = app.jobs.iter().map(|j| &j.alert);
            checks
                .chain(containers)
                .chain(certificates)
                .chain(services)
                .chain(jobs)
        });
        [&host.cpu.alert, &host.memory.alert]
            .into_iter()
            .chain(host.storage.iter().map(|m| &m.metric.alert))
            .chain(host.temperature.iter().map(|s| &s.alert))
            .chain(apps)
            .flatten()
    }

    pub(super) fn alerts_mut(&mut self) -> Vec<&mut Alert> {
        let host = &mut self.host;
        let mut alerts = vec![&mut host.cpu.alert, &mut host.memory.alert];
        alerts.extend(host.storage.iter_mut().map(|m| &mut m.metric.alert));
        alerts.extend(host.temperature.iter_mut().map(|s| &mut s.alert));
        for app in &mut self.apps {
            alerts.extend(app.health_checks.iter_mut().map(|c| &mut c.alert));
            alerts.extend(app.containers.iter_mut().map(|c| &mut c.alert));
            alerts.extend(app.certificates.iter_mut().map(|c| &mut c.alert));
            alerts.extend(app.services.iter_mut().map(|s| &mut s.alert));
            alerts.extend(app.jobs.iter_mut().map(|j| &mut j.alert));
        }
        alerts.into_iter().flatten().collect()
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "listen: {}", self.listen)?;
        writeln!(f, "context: {}", self.context)?;
        writeln!(f, "host.cpu: {}", self.host.cpu)?;
        writeln!(f, "host.memory: {}", self.host.memory)?;
        for mount in &self.host.storage {
            writeln!(f, "host.storage {}: {}", mount.path.display(), mount.metric)?;
        }
        for sensor in &self.host.temperature {
            writeln!(
                f,
                "host.temperature \"{}\": {}, every {}",
                sensor.id,
                sensor.name,
                Dur::from(sensor.every)
            )?;
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
        writeln!(
            f,
            "certificates: every {}, warn before {}",
            Dur::from(self.certificates.every),
            Dur::from(self.certificates.warn_before),
        )?;
        writeln!(f, "services: every {}", Dur::from(self.services.every))?;
        for app in &self.apps {
            write!(f, "apps.{}: {}", app.id, app.name)?;
            if !app.containers.is_empty() {
                let names: Vec<_> = app.containers.iter().map(|c| c.name.as_str()).collect();
                write!(f, ", containers {}", names.join(", "))?;
            }
            if !app.certificates.is_empty() {
                let hosts: Vec<_> = app
                    .certificates
                    .iter()
                    .map(|c| c.target.to_string())
                    .collect();
                write!(f, ", certificates {}", hosts.join(", "))?;
            }
            if !app.services.is_empty() {
                let services: Vec<_> = app.services.iter().map(|s| s.name.as_str()).collect();
                write!(f, ", services {}", services.join(", "))?;
            }
            writeln!(f)?;
            for check in &app.health_checks {
                writeln!(f, "apps.{}.{}: {check}", app.id, check.name)?;
            }
            for job in &app.jobs {
                writeln!(f, "apps.{}.jobs.{}: {job}", app.id, job.name)?;
            }
        }
        for notifier in &self.notifiers {
            writeln!(f, "notifiers.{}: {}", notifier.name, notifier.kind)?;
        }
        if let Some(heartbeat) = &self.heartbeat {
            writeln!(f, "heartbeat: {heartbeat}")?;
        }
        for alert in self.alerts() {
            writeln!(f, "alert \"{}\": {alert}", alert.id)?;
        }
        Ok(())
    }
}

impl fmt::Display for NotifierKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ntfy(ntfy) => {
                write!(f, "ntfy topic {}", ntfy.topic)?;
                match &ntfy.address {
                    Some(address) => write!(f, " at {address}")?,
                    None => write!(f, " at the default server")?,
                }
                if ntfy.token.is_some() {
                    write!(f, ", with a token")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for Alert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.label, self.severity)?;
        if let Some(threshold) = &self.threshold {
            let unit = threshold.unit;
            write!(f, ", above {}{unit}", threshold.above)?;
            if let Some(below) = threshold.resolve_below {
                write!(f, " until below {below}{unit}")?;
            }
        }
        if !self.after.is_zero() {
            write!(f, ", for {}", Dur::from(self.after))?;
        }
        if let Some(limit) = self.restarts {
            write!(
                f,
                ", or more than {} restarts within {}",
                limit.above,
                Dur::from(limit.within)
            )?;
        }
        if !self.parents.is_empty() {
            write!(f, ", depends on {}", self.parents.join(", "))?;
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

/// Without the URL's path and query, the header values or the body: any of
/// them can hold the secret that identifies the check.
impl fmt::Display for Heartbeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = self.url.to_string();
        let origin = &url[..url.len() - self.url.target.len()];
        write!(
            f,
            "{} {origin}/…, every {}",
            self.method,
            Dur::from(self.every)
        )?;
        if !self.headers.is_empty() {
            let names: Vec<&str> = self.headers.iter().map(|(name, _)| name.as_str()).collect();
            write!(f, ", headers {}", names.join(", "))?;
        }
        if let Some(body) = &self.body {
            write!(f, ", a {}-byte body", body.len())?;
        }
        Ok(())
    }
}

impl fmt::Display for AppJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "schedule \"{}\"", self.schedule)?;
        match self.max_duration {
            Some(max) => write!(f, ", runs up to {}", Dur::from(max))?,
            None => write!(f, ", runs until the next is due")?,
        }
        let mut due = Some(now());
        let next: Vec<String> = (0..3)
            .map_while(|_| {
                due = self.schedule.next_after(due?);
                due.map(format_local)
            })
            .collect();
        write!(f, ", next due {}", next.join(", "))
    }
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "every {}", Dur::from(self.every))
    }
}
