//! Types shared by collectors, the store and the web UI.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// What a series belongs to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityId {
    Host,
    /// A configured storage path.
    Mount(PathBuf),
    /// A configured temperature sensor, by `driver/label`.
    Sensor(String),
    /// Keyed by name, which survives the container being recreated.
    Container(String),
    /// An app's health check, by app ID and check name.
    HealthCheck {
        app: String,
        check: String,
    },
    /// An app's TLS certificate, by app ID and `host[:port]`.
    Certificate {
        app: String,
        host: String,
    },
    /// An app's systemd or launchd service, by app ID and name.
    Service {
        app: String,
        service: String,
    },
    /// An app's command, by app ID and name.
    Command {
        app: String,
        command: String,
    },
}

/// Which metric of an entity a series holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricKind {
    /// Percent of all host cores, 0..=100.
    Cpu,
    /// Percent of RAM held by programs, cache excluded, 0..=100.
    Memory,
    /// Bytes held, cache excluded.
    MemoryBytes,
    /// Percent of the filesystem used, as `df` reports it, 0..=100.
    Storage,
    /// Degrees Celsius.
    Temperature,
    /// Milliseconds until a successful response's headers arrived.
    Latency,
    /// Restarts within the service's restart window.
    Restarts,
    /// Seconds a command's run took.
    Runtime,
}

/// State of an entity that can come and go, or fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Containers.
    Running,
    Paused,
    /// Was running earlier, isn't anymore.
    Gone {
        since: Timestamp,
    },
    /// Health checks, services and commands. `since` is the first attempt
    /// of the current streak.
    Up {
        since: Timestamp,
    },
    /// Commands only: exited with one of their `warnOn` codes.
    Warn {
        since: Timestamp,
    },
    Down {
        since: Timestamp,
    },
}

/// Unix time in seconds.
pub type Timestamp = u32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub at: Timestamp,
    pub value: f32,
}

impl Sample {
    pub fn now(value: f32) -> Self {
        Self { at: now(), value }
    }
}

pub fn now() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as Timestamp)
}
