//! Turns `RawConfig` into the final `Config`: applies defaults and runs
//! checks that serde can't express. Collects every problem instead of
//! stopping at the first one.

use std::time::Duration;

use super::raw::{RawConfig, RawMetric};
use super::resolved::{Config, DockerConfig, HostConfig, Metric, Mount};
use super::types::Dur;

const DEFAULT_EVERY: Duration = Duration::from_secs(60);
const MIN_EVERY: Duration = Duration::from_secs(1);

pub fn resolve(raw: RawConfig) -> Result<Config, Vec<String>> {
    let mut errors = Vec::new();
    let host = raw.host.unwrap_or_default();

    let cpu = metric(
        "host.cpu",
        host.cpu.unwrap_or_default(),
        DEFAULT_EVERY,
        &mut errors,
    );
    let memory = metric(
        "host.memory",
        host.memory.unwrap_or_default(),
        DEFAULT_EVERY,
        &mut errors,
    );

    let raw_storage = host.storage.unwrap_or_default();
    let storage_every = raw_storage.every.map_or(DEFAULT_EVERY, Dur::get);
    let mut storage = Vec::new();
    for (path, raw_metric) in raw_storage.mounts.unwrap_or_default() {
        let at = format!("host.storage.mounts.\"{}\"", path.display());
        if !path.is_absolute() {
            errors.push(format!("{at}: mount point must be an absolute path"));
        } else if !path.is_dir() {
            errors.push(format!("{at}: no such directory"));
        }
        let metric = metric(&at, raw_metric, storage_every, &mut errors);
        storage.push(Mount { path, metric });
    }

    let docker = raw.docker.map(|raw_docker| {
        if let Some(socket) = &raw_docker.socket
            && !socket.exists()
        {
            errors.push(format!(
                "docker.socket: {} does not exist",
                socket.display()
            ));
        }
        DockerConfig {
            every: every("docker", raw_docker.every, DEFAULT_EVERY, &mut errors),
            socket: raw_docker.socket,
        }
    });

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Config {
        listen: raw.listen,
        host: HostConfig {
            cpu,
            memory,
            storage,
        },
        docker,
    })
}

fn every(at: &str, raw: Option<Dur>, default: Duration, errors: &mut Vec<String>) -> Duration {
    let every = raw.map_or(default, Dur::get);
    if every < MIN_EVERY {
        errors.push(format!(
            "{at}.every: must be at least {}",
            Dur::from(MIN_EVERY)
        ));
    }
    every
}

fn metric(at: &str, raw: RawMetric, default_every: Duration, errors: &mut Vec<String>) -> Metric {
    let every = every(at, raw.every, default_every, errors);
    if let Some(alert) = raw.alert
        && alert.duration.get() < every
    {
        errors.push(format!(
            "{at}.alert.for ({}) is shorter than {at}.every ({}), it would fire on a single sample",
            alert.duration,
            Dur::from(every),
        ));
    }
    Metric {
        every,
        alert: raw.alert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn applies_defaults() {
        let config = resolve(parse(r#"{ "listen": "127.0.0.1:9100" }"#)).unwrap();
        assert_eq!(config.host.cpu.every, DEFAULT_EVERY);
        assert!(config.host.cpu.alert.is_none());
        assert!(config.host.storage.is_empty());
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
    fn rejects_unknown_fields() {
        let result: Result<RawConfig, _> =
            serde_json::from_str(r#"{ "listen": "127.0.0.1:9100", "hots": {} }"#);
        assert!(result.is_err());
    }
}
