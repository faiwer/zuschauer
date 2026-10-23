use std::time::{Duration, Instant};

use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::config::{App, HealthCheck};
use crate::http::get_head;
use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp, now};
use crate::store::store;

use super::verdict::verdict;

pub async fn run(app: &'static App, check: &'static HealthCheck) {
    let entity = EntityId::HealthCheck {
        app: app.id.clone(),
        check: check.name.clone(),
    };
    let label = format!("{}/{}", app.id, check.name);
    let mut status = None;

    let mut ticker = interval(check.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let result = attempt(check).await;
        let next = streak(status, result.is_ok(), now());
        status = Some(next);
        let mut store = store();
        store.set_status(entity.clone(), next);
        match result {
            Ok(latency) => {
                let ms = latency.as_secs_f32() * 1000.0;
                store.set_error(entity.clone(), None);
                store.push(entity.clone(), MetricKind::Latency, Sample::now(ms));
                eprintln!("app {label}: up, responded in {ms:.0} ms");
            }
            Err(error) => {
                eprintln!("app {label}: down, {error}");
                store.set_error(entity.clone(), Some(error));
            }
        }
    }
}

/// Keeps `since` while the outcome stays the same.
fn streak(prev: Option<Status>, up: bool, at: Timestamp) -> Status {
    match (prev, up) {
        (Some(Status::Up { since }), true) => Status::Up { since },
        (Some(Status::Down { since }), false) => Status::Down { since },
        (_, true) => Status::Up { since: at },
        (_, false) => Status::Down { since: at },
    }
}

/// How long the response's headers took to arrive, or why it failed.
async fn attempt(check: &HealthCheck) -> Result<Duration, String> {
    let started = Instant::now();
    let head = timeout(check.timeout, get_head(&check.url))
        .await
        .map_err(|_| {
            format!(
                "no response within {}",
                humantime::format_duration(check.timeout)
            )
        })?
        .map_err(|e| e.to_string())?;
    let latency = started.elapsed();
    verdict(check, &head)?;
    Ok(latency)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_since_within_a_streak() {
        let up = streak(None, true, 10);
        assert_eq!(up, Status::Up { since: 10 });
        assert_eq!(streak(Some(up), true, 20), Status::Up { since: 10 });
        let down = streak(Some(up), false, 30);
        assert_eq!(down, Status::Down { since: 30 });
        assert_eq!(streak(Some(down), false, 40), Status::Down { since: 30 });
        assert_eq!(streak(Some(down), true, 50), Status::Up { since: 50 });
    }
}
