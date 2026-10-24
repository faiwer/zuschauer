//! Sends the configured request every so often, so an outside service like
//! healthchecks.io notices when zuschauer, or its machine, stops.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::config::{Dur, Heartbeat};
use crate::http::{describe, request};
use crate::model::{Timestamp, now};

/// Capped at the heartbeat's `every` when that's shorter.
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct Status {
    /// The last request that got a 2xx.
    pub sent: Option<Timestamp>,
    /// Since when requests fail, and why the latest one did.
    pub failing: Option<(Timestamp, String)>,
}

static STATUS: Mutex<Status> = Mutex::new(Status {
    sent: None,
    failing: None,
});

pub fn status() -> Status {
    STATUS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// The first request goes out right away, so a restart shows up at once.
pub fn spawn(heartbeat: Option<&'static Heartbeat>) {
    if let Some(heartbeat) = heartbeat {
        tokio::spawn(run(heartbeat));
    }
}

async fn run(heartbeat: &'static Heartbeat) {
    let headers: Vec<(&str, &str)> = heartbeat
        .headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let mut ticker = interval(heartbeat.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let result = send(heartbeat, &headers).await;
        record(result, now());
    }
}

/// Logs only changes.
fn record(result: Result<(), String>, at: Timestamp) {
    let mut status = STATUS.lock().unwrap_or_else(PoisonError::into_inner);
    match result {
        Ok(()) => {
            if status.sent.is_none() || status.failing.is_some() {
                eprintln!("heartbeat: sent");
            }
            status.sent = Some(at);
            status.failing = None;
        }
        Err(error) => {
            let since = match &status.failing {
                Some((since, _)) => *since,
                None => {
                    eprintln!("heartbeat: failing, {error}");
                    at
                }
            };
            status.failing = Some((since, error));
        }
    }
}

async fn send(heartbeat: &Heartbeat, headers: &[(&str, &str)]) -> Result<(), String> {
    let limit = TIMEOUT.min(heartbeat.every);
    let body = heartbeat.body.as_deref().map(str::as_bytes);
    let response = timeout(
        limit,
        request(&heartbeat.url, &heartbeat.method, headers, body),
    )
    .await
    .map_err(|_| format!("no response within {}", Dur::from(limit)))?
    .map_err(|e| describe(&e))?;
    match response.head.status {
        200..300 => Ok(()),
        status => Err(format!("HTTP {status}")),
    }
}
