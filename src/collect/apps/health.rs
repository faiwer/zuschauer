use std::time::{Duration, Instant};

use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::config::{App, HealthCheck};
use crate::http::{describe, get_head};
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

use super::outcome::record;
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
        let outcome = attempt(check).await.map(|latency| {
            let ms = latency.as_secs_f32() * 1000.0;
            eprintln!("app {label}: up, responded in {ms:.0} ms");
            store().push(entity.clone(), MetricKind::Latency, Sample::now(ms));
        });
        if let Err(error) = &outcome {
            eprintln!("app {label}: down, {error}");
        }
        status = Some(record(&entity, status, outcome));
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
        .map_err(|e| describe(&e))?;
    let latency = started.elapsed();
    verdict(check, &head)?;
    Ok(latency)
}
