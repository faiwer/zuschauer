use tokio::time::{MissedTickBehavior, interval};

use crate::alerts::{Observation, observe};
use crate::config::Metric;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

#[cfg(target_os = "linux")]
use super::linux::read;
#[cfg(target_os = "macos")]
use super::macos::read;

pub async fn run(config: &'static Metric) {
    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let usage = match read() {
            Ok(usage) => usage,
            Err(e) => {
                eprintln!("memory: {e}");
                continue;
            }
        };
        let Some(percent) = usage.percent() else {
            eprintln!("memory: implausible reading {usage:?}");
            continue;
        };
        store().push(EntityId::Host, MetricKind::Memory, Sample::now(percent));
        eprintln!(
            "memory: {percent:.1}% ({} / {} MiB)",
            usage.used >> 20,
            usage.total >> 20
        );
        if let Some(alert) = &config.alert {
            observe(alert, Observation::Value(percent));
        }
    }
}
