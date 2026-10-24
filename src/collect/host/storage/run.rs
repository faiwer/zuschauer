use std::io;

use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};

use crate::alerts::{Observation, observe};
use crate::config::Mount;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

#[cfg(target_os = "linux")]
use super::linux::read;
#[cfg(target_os = "macos")]
use super::macos::read;

pub async fn run(mount: &'static Mount) {
    let entity = EntityId::Mount(mount.path.clone());
    let mut ticker = interval(mount.metric.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        // A hung network mount can block the call forever, so it runs on the
        // blocking pool instead of the runtime's only thread.
        let result = spawn_blocking(|| read(&mount.path))
            .await
            .unwrap_or_else(|e| Err(io::Error::other(e)));
        let blocks = match result {
            Ok(blocks) => blocks,
            Err(e) => {
                eprintln!("storage {}: {e}", mount.path.display());
                continue;
            }
        };
        let Some(percent) = blocks.percent() else {
            eprintln!(
                "storage {}: implausible reading {blocks:?}",
                mount.path.display()
            );
            continue;
        };
        store().push(entity.clone(), MetricKind::Storage, Sample::now(percent));
        if let Some(alert) = &mount.metric.alert {
            observe(alert, Observation::Value(percent));
        }
    }
}
