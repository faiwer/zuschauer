use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::config::{Alert, Severity};
use crate::model::{Timestamp, now};
use crate::notify;

use super::engine::{Change, Engine, Observation};
use super::message;

static ENGINE: Mutex<Engine> = Mutex::new(Engine::new());

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Feeds what a collector saw into `alert`, and notifies when that makes it,
/// or alerts it was holding back, fire or resolve.
pub fn observe(alert: &'static Alert, observation: Observation) {
    let at = now();
    let changes = engine().observe(alert, observation, at);
    for (alert, change) in changes {
        let message = match change {
            Change::Fired {
                since,
                detail,
                severity,
            } => {
                eprintln!("alert {}: firing, {severity}, {detail}", alert.id);
                message::fired(alert, severity, &detail, since, at)
            }
            Change::Resolved {
                since,
                detail,
                severity,
            } => {
                eprintln!("alert {}: resolved", alert.id);
                message::resolved(alert, severity, detail, since, at)
            }
        };
        notify::send(message);
    }
}

pub struct Firing {
    pub alert: &'static Alert,
    /// When the problem started, not when the alert fired.
    pub since: Timestamp,
    pub detail: String,
    /// Can be milder than the alert's, for a warning.
    pub severity: Severity,
}

/// Sorted by alert ID.
pub fn firing() -> Vec<Firing> {
    engine()
        .firing()
        .map(|(alert, problem)| Firing {
            alert,
            since: problem.since,
            detail: problem.detail.clone(),
            severity: problem.severity,
        })
        .collect()
}
