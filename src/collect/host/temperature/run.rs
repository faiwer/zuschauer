use std::io;
use std::path::PathBuf;

use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};

use crate::alerts::{Observation, observe};
use crate::config::Sensor;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

use super::hwmon::read;

pub async fn run(sensor: &'static Sensor, path: PathBuf) {
    let entity = EntityId::Sensor(sensor.id.clone());
    let mut ticker = interval(sensor.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        // Some drivers ask the hardware on every read, which can stall, so
        // the read runs on the blocking pool instead of the runtime's only
        // thread.
        let path = path.clone();
        let result = spawn_blocking(move || read(&path))
            .await
            .unwrap_or_else(|e| Err(io::Error::other(e)));
        match result {
            Ok(celsius) => {
                let mut store = store();
                store.push(
                    entity.clone(),
                    MetricKind::Temperature,
                    Sample::now(celsius),
                );
                store.set_error(entity.clone(), None);
                drop(store);
                eprintln!("temperature {}: {celsius:.1}C", sensor.name);
                if let Some(alert) = &sensor.alert {
                    observe(alert, Observation::Value(celsius));
                }
            }
            Err(e) => {
                eprintln!("temperature {}: {e}", sensor.name);
                store().set_error(entity.clone(), Some(e.to_string()));
            }
        }
    }
}
