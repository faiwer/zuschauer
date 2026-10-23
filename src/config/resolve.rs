//! Turns `RawConfig` into the final `Config`: applies defaults and runs
//! checks that serde can't express. Collects every problem instead of
//! stopping at the first one.

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use super::raw::{RawApp, RawCertificates, RawConfig, RawHealthCheck, RawMetric, RawSensor};
use super::resolved::{
    App, CertificatesConfig, Config, DockerConfig, HealthCheck, HostConfig, Metric, Mount, Sensor,
};
use super::types::Dur;

const DEFAULT_EVERY: Duration = Duration::from_secs(60);
const MIN_EVERY: Duration = Duration::from_secs(1);
/// Capped at the check's `every` when that's shorter.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_EXPECT_CODE: u16 = 200;
/// Certificates change rarely; this only has to catch a renewal that stopped.
const DEFAULT_CERTIFICATES_EVERY: Duration = Duration::from_secs(6 * 3600);
const DEFAULT_WARN_BEFORE: Duration = Duration::from_secs(14 * 86400);

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

    let temperature = host
        .temperature
        .unwrap_or_default()
        .into_iter()
        .map(|(id, raw_sensor)| sensor(id, raw_sensor, &mut errors))
        .collect();

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

    let certificates = certificates(raw.certificates.unwrap_or_default(), &mut errors);
    let apps = apps(raw.apps.unwrap_or_default(), docker.is_some(), &mut errors);

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Config {
        listen: raw.listen,
        host: HostConfig {
            cpu,
            memory,
            storage,
            temperature,
        },
        docker,
        certificates,
        apps,
    })
}

fn certificates(raw: RawCertificates, errors: &mut Vec<String>) -> CertificatesConfig {
    let warn_before = raw.warn_before.map_or(DEFAULT_WARN_BEFORE, Dur::get);
    if warn_before.is_zero() {
        errors.push("certificates.warnBefore: must be more than 0".to_owned());
    }
    CertificatesConfig {
        every: every(
            "certificates",
            raw.every,
            DEFAULT_CERTIFICATES_EVERY,
            errors,
        ),
        warn_before,
    }
}

fn apps(raw: BTreeMap<String, RawApp>, docker: bool, errors: &mut Vec<String>) -> Vec<App> {
    raw.into_iter()
        .map(|(id, raw_app)| app(id, raw_app, docker, errors))
        .collect()
}

fn app(id: String, raw: RawApp, docker: bool, errors: &mut Vec<String>) -> App {
    let at = format!("apps.{id}");
    if id.is_empty() {
        errors.push("apps: an app ID can't be empty".to_owned());
    }

    let containers = raw.containers.unwrap_or_default();
    if !containers.is_empty() && !docker {
        errors.push(format!(
            "{at}.containers: containers aren't monitored, add a \"docker\" section"
        ));
    }
    let mut seen = HashSet::new();
    for (i, name) in containers.iter().enumerate() {
        if name.is_empty() {
            errors.push(format!("{at}.containers[{i}]: can't be empty"));
        } else if !seen.insert(name) {
            errors.push(format!("{at}.containers[{i}]: \"{name}\" is listed twice"));
        }
    }

    let certificates = raw.certificates.unwrap_or_default();
    for (i, host) in certificates.iter().enumerate() {
        if certificates[..i].contains(host) {
            errors.push(format!(
                "{at}.certificates[{i}]: \"{host}\" is listed twice"
            ));
        }
    }

    let raw_checks = raw.health_checks.unwrap_or_default();
    if raw_checks.is_empty() && containers.is_empty() && certificates.is_empty() {
        errors.push(format!(
            "{at}: nothing to monitor, add healthChecks, containers or certificates"
        ));
    }
    let mut names = HashSet::new();
    let health_checks = raw_checks
        .into_iter()
        .enumerate()
        .map(|(i, raw_check)| {
            let check = health_check(&format!("{at}.healthChecks[{i}]"), raw_check, errors);
            if !names.insert(check.name.clone()) {
                errors.push(format!(
                    "{at}.healthChecks[{i}]: another check is also named \"{}\", give one a \"name\"",
                    check.name
                ));
            }
            check
        })
        .collect();

    App {
        name: raw.name.unwrap_or_else(|| id.clone()),
        id,
        containers,
        health_checks,
        certificates,
    }
}

fn health_check(at: &str, raw: RawHealthCheck, errors: &mut Vec<String>) -> HealthCheck {
    let every = every(at, raw.every, DEFAULT_EVERY, errors);

    let timeout = raw.timeout.map_or(DEFAULT_TIMEOUT.min(every), Dur::get);
    if timeout.is_zero() {
        errors.push(format!("{at}.timeout: must be more than 0"));
    } else if timeout > every {
        errors.push(format!(
            "{at}.timeout ({}) is longer than {at}.every ({})",
            Dur::from(timeout),
            Dur::from(every),
        ));
    }

    let expect_code = raw.expect_code.unwrap_or(DEFAULT_EXPECT_CODE);
    if !(100..=599).contains(&expect_code) {
        errors.push(format!(
            "{at}.expectCode: {expect_code} is not an HTTP status code"
        ));
    }

    if raw.content_type.as_deref().is_some_and(str::is_empty) {
        errors.push(format!(
            "{at}.contentType: can't be empty, leave it out instead"
        ));
    }

    HealthCheck {
        name: raw.name.unwrap_or_else(|| raw.url.to_string()),
        url: raw.url,
        every,
        timeout,
        expect_code,
        content_type: raw.content_type,
    }
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
    let every = every(&at, raw.every, DEFAULT_EVERY, errors);
    if let Some(alert) = &raw.alert {
        alert_for(&at, alert.duration, every, errors);
    }
    Sensor {
        name: raw.name.unwrap_or_else(|| id.clone()),
        id,
        every,
        alert: raw.alert,
    }
}

fn metric(at: &str, raw: RawMetric, default_every: Duration, errors: &mut Vec<String>) -> Metric {
    let every = every(at, raw.every, default_every, errors);
    if let Some(alert) = &raw.alert {
        alert_for(at, alert.duration, every, errors);
    }
    Metric {
        every,
        alert: raw.alert,
    }
}

fn alert_for(at: &str, duration: Dur, every: Duration, errors: &mut Vec<String>) {
    if duration.get() < every {
        errors.push(format!(
            "{at}.alert.for ({duration}) is shorter than {at}.every ({}), it would fire on a single sample",
            Dur::from(every),
        ));
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
    fn applies_health_check_defaults() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": {
                    "web": {
                        "healthChecks": [
                            { "url": "https://example.com/health" },
                            { "name": "fast", "url": "http://127.0.0.1:8080/", "every": "5s" }
                        ]
                    }
                }
            }"#,
        ))
        .unwrap();
        let app = &config.apps[0];
        assert_eq!(app.name, "web");
        let [slow, fast] = &app.health_checks[..] else {
            panic!("{:#?}", app.health_checks);
        };
        assert_eq!(slow.name, "https://example.com/health");
        assert_eq!((slow.every, slow.timeout), (DEFAULT_EVERY, DEFAULT_TIMEOUT));
        assert_eq!(slow.expect_code, 200);
        assert_eq!(fast.timeout, Duration::from_secs(5));
    }

    #[test]
    fn collects_health_check_errors() {
        let raw = parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": {
                    "empty": {},
                    "web": {
                        "healthChecks": [
                            { "url": "http://a/", "every": "10s", "timeout": "20s" },
                            { "url": "http://a/", "expectCode": 42, "contentType": "" }
                        ]
                    }
                }
            }"#,
        );
        let errors = resolve(raw).unwrap_err();
        // Nothing to monitor, timeout > every, duplicate name, expectCode,
        // empty contentType.
        assert_eq!(errors.len(), 5, "{errors:#?}");
    }

    #[test]
    fn checks_app_containers() {
        let ok = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "docker": {},
                "apps": { "db": { "containers": ["db-1"] } }
            }"#,
        ))
        .unwrap();
        assert_eq!(ok.apps[0].containers, ["db-1"]);

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "db": { "containers": ["db-1", "db-1", ""] } }
            }"#,
        ))
        .unwrap_err();
        // No docker section, listed twice, empty name.
        assert_eq!(errors.len(), 3, "{errors:#?}");
    }

    #[test]
    fn checks_certificates() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "site": { "certificates": ["example.com", "example.com:8443"] } }
            }"#,
        ))
        .unwrap();
        assert_eq!(config.certificates.every, DEFAULT_CERTIFICATES_EVERY);
        assert_eq!(config.apps[0].certificates.len(), 2);

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "certificates": { "warnBefore": "0s" },
                "apps": { "site": { "certificates": ["example.com", "example.com:443"] } }
            }"#,
        ))
        .unwrap_err();
        // warnBefore of 0, the same host twice (443 is the default).
        assert_eq!(errors.len(), 2, "{errors:#?}");
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

    #[test]
    fn rejects_bad_urls_while_parsing() {
        let result: Result<RawConfig, _> = serde_json::from_str(
            r#"{ "listen": "127.0.0.1:9100", "apps": { "a": { "healthChecks": [{ "url": "ftp://a/" }] } } }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_fields() {
        let result: Result<RawConfig, _> =
            serde_json::from_str(r#"{ "listen": "127.0.0.1:9100", "hots": {} }"#);
        assert!(result.is_err());
    }
}
