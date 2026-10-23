use std::path::Path;

use crate::config::Sensor;
use crate::model::EntityId;
use crate::store::store;

use super::hwmon::{ROOT, discover};
use super::run::run;

pub fn spawn(sensors: &'static [Sensor]) {
    if sensors.is_empty() {
        return;
    }
    let found = discover(Path::new(ROOT)).unwrap_or_else(|e| {
        eprintln!("temperature: can't list sensors in {ROOT}: {e}");
        Vec::new()
    });
    for sensor in sensors {
        let matching: Vec<_> = found.iter().filter(|f| f.id == sensor.id).collect();
        match matching[..] {
            [found] => {
                tokio::spawn(run(sensor, found.path.clone()));
            }
            [] => unavailable(sensor, "not found, see `zuschauer --sensors`".to_owned()),
            _ => unavailable(
                sensor,
                format!("{} sensors share this name", matching.len()),
            ),
        }
    }
}

fn unavailable(sensor: &Sensor, error: String) {
    eprintln!("temperature {}: {error}", sensor.id);
    store().set_error(EntityId::Sensor(sensor.id.clone()), Some(error));
}
