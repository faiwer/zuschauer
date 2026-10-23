use std::time::Duration;

use crate::config::raw::{RawHost, RawMetric, RawSensor};
use crate::config::resolved::{HostConfig, Metric, Mount, Sensor};
use crate::config::types::Dur;

use super::alerts::{AlertNames, threshold_alert};
use super::every::{DEFAULT_EVERY, every};

pub(super) fn host(raw: RawHost, errors: &mut Vec<String>) -> HostConfig {
    let cpu = metric(
        "host.cpu",
        raw.cpu.unwrap_or_default(),
        DEFAULT_EVERY,
        AlertNames::new("cpu", "CPU"),
        errors,
    );
    let memory = metric(
        "host.memory",
        raw.memory.unwrap_or_default(),
        DEFAULT_EVERY,
        AlertNames::new("memory", "Memory"),
        errors,
    );

    let raw_storage = raw.storage.unwrap_or_default();
    let storage_every = raw_storage.every.map_or(DEFAULT_EVERY, Dur::get);
    let mut storage = Vec::new();
    for (path, raw_metric) in raw_storage.mounts.unwrap_or_default() {
        let at = format!("host.storage.mounts.\"{}\"", path.display());
        if !path.is_absolute() {
            errors.push(format!("{at}: mount point must be an absolute path"));
        } else if !path.is_dir() {
            errors.push(format!("{at}: no such directory"));
        }
        let names = AlertNames::new(
            format!("storage:{}", path.display()),
            format!("Storage {}", path.display()),
        );
        let metric = metric(&at, raw_metric, storage_every, names, errors);
        storage.push(Mount { path, metric });
    }

    let temperature = raw
        .temperature
        .unwrap_or_default()
        .into_iter()
        .map(|(id, raw_sensor)| sensor(id, raw_sensor, errors))
        .collect();

    HostConfig {
        cpu,
        memory,
        storage,
        temperature,
    }
}

fn metric(
    at: &str,
    raw: RawMetric,
    default_every: Duration,
    names: AlertNames,
    errors: &mut Vec<String>,
) -> Metric {
    let every = every(at, raw.every, default_every, errors);
    let alert = raw
        .alert
        .map(|raw_alert| threshold_alert(at, raw_alert, every, "%", names, errors));
    Metric { every, alert }
}

fn sensor(id: String, raw: RawSensor, errors: &mut Vec<String>) -> Sensor {
    let at = format!("host.temperature.\"{id}\"");
    if !id
        .split_once('/')
        .is_some_and(|(driver, label)| !driver.is_empty() && !label.is_empty())
    {
        errors.push(format!(
            "{at}: expected \"driver/label\" as `zuschauer --sensors` lists it"
        ));
    }
    if raw.name.as_deref().is_some_and(str::is_empty) {
        errors.push(format!("{at}.name: can't be empty, leave it out instead"));
    }
    let name = raw.name.unwrap_or_else(|| id.clone());
    let every = every(&at, raw.every, DEFAULT_EVERY, errors);
    let alert = raw.alert.map(|raw_alert| {
        let names = AlertNames::new(format!("temperature:{id}"), format!("Temperature {name}"));
        threshold_alert(&at, raw_alert, every, "°C", names, errors)
    });
    Sensor {
        id,
        name,
        every,
        alert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn mount_every_overrides_storage_every() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "host": {
                    "storage": {
                        "every": "5m",
                        "mounts": { "/": {}, "/tmp": { "every": "30s" } }
                    }
                }
            }"#,
        ))
        .unwrap();
        let every: Vec<_> = config.host.storage.iter().map(|m| m.metric.every).collect();
        assert_eq!(every, [Duration::from_secs(300), Duration::from_secs(30)]);
    }

    #[test]
    fn collects_all_errors() {
        let raw = parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "host": {
                    "cpu": { "every": "60s", "alert": { "above": "85%", "for": "10s" } },
                    "memory": { "every": "100ms" },
                    "storage": { "mounts": { "relative/path": {} } }
                }
            }"#,
        );
        let errors = resolve(raw).unwrap_err();
        assert_eq!(errors.len(), 3, "{errors:#?}");
    }

    #[test]
    fn checks_sensors() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "host": {
                    "temperature": {
                        "chip/Sensor A/B": { "name": "Board", "alert": { "above": "70C", "for": "10m" } },
                        "chip/temp2": { "every": "10s" }
                    }
                }
            }"#,
        ))
        .unwrap();
        let [labeled, plain] = &config.host.temperature[..] else {
            panic!("{:#?}", config.host.temperature);
        };
        assert_eq!((&*labeled.name, labeled.every), ("Board", DEFAULT_EVERY));
        assert_eq!(
            (&*plain.name, plain.every),
            ("chip/temp2", Duration::from_secs(10))
        );

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "host": {
                    "temperature": {
                        "chip": {},
                        "/temp1": {},
                        "chip/temp1": { "name": "", "every": "1m", "alert": { "above": "70C", "for": "10s" } }
                    }
                }
            }"#,
        ))
        .unwrap_err();
        // Two bad IDs, empty name, alert.for < every.
        assert_eq!(errors.len(), 4, "{errors:#?}");
    }
}
