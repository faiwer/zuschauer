//! Types shared by collectors, the store and the web UI.

use std::time::{SystemTime, UNIX_EPOCH};

/// What a series belongs to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityId {
    Host,
}

/// Which metric of an entity a series holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricKind {
    /// Percent of all cores, 0..=100.
    Cpu,
    /// Percent of RAM held by programs, cache excluded, 0..=100.
    Memory,
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
