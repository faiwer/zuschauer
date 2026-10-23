use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::config::Alert;
use crate::model::{Timestamp, now};
use crate::notify;

use super::engine::{Change, Engine, Observation};
use super::message;

static ENGINE: Mutex<Engine> = Mutex::new(Engine::new());

fn engine() -> MutexGuard<'static, Engine> {
    ENGINE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Feeds what a collector saw into `alert`, and notifies when that makes it
/// fire or resolve.
pub fn observe(alert: &'static Alert, observation: Observation) {
    let at = now();
    let change = engine().observe(alert, observation, at);
    let message = match change {
        None => return,
        Some(Change::Fired { since, detail }) => {
            eprintln!("alert {}: firing, {detail}", alert.id);
            message::fired(alert, &detail, since, at)
        }
        Some(Change::Resolved { since }) => {
            eprintln!("alert {}: resolved", alert.id);
            message::resolved(alert, since, at)
        }
    };
    notify::send(message);
}

pub struct Firing {
    pub alert: &'static Alert,
    /// When the problem started, not when the alert fired.
    pub since: Timestamp,
    pub detail: String,
}

/// Sorted by alert ID.
pub fn firing() -> Vec<Firing> {
    engine()
        .firing()
        .map(|(alert, problem)| Firing {
            alert,
            since: problem.since,
            detail: problem.detail.clone(),
        })
        .collect()
}
