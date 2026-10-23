use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use crate::config::raw::{RawApp, RawHealthCheck};
use crate::config::resolved::{App, AppCertificate, AppContainer, HealthCheck};
use crate::config::types::Dur;
use crate::http::HostPort;

use super::alerts::{AlertNames, state_alert, watched_alert};
use super::every::{DEFAULT_EVERY, every};

/// Capped at the check's `every` when that's shorter.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_EXPECT_CODE: u16 = 200;

pub(super) fn apps(
    raw: BTreeMap<String, RawApp>,
    docker: bool,
    errors: &mut Vec<String>,
) -> Vec<App> {
    raw.into_iter()
        .map(|(id, raw_app)| app(id, raw_app, docker, errors))
        .collect()
}

fn app(id: String, raw: RawApp, docker: bool, errors: &mut Vec<String>) -> App {
    let at = format!("apps.{id}");
    if id.is_empty() {
        errors.push("apps: an app ID can't be empty".to_owned());
    }
    let name = raw.name.unwrap_or_else(|| id.clone());

    let raw_containers = raw.containers.unwrap_or_default();
    if !raw_containers.is_empty() && !docker {
        errors.push(format!(
            "{at}.containers: containers aren't monitored, add a \"docker\" section"
        ));
    }
    let containers: Vec<_> = raw_containers
        .into_iter()
        .map(|(container, raw)| {
            let at = format!("{at}.containers.\"{container}\"");
            if container.is_empty() {
                errors.push(format!("{at}: a container name can't be empty"));
            }
            let names = AlertNames::new(
                format!("{id}/{container}"),
                format!("{name} container {container}"),
            );
            AppContainer {
                alert: watched_alert(&at, raw, names, errors),
                name: container,
            }
        })
        .collect();

    let mut certificates: Vec<AppCertificate> = Vec::new();
    for (host, raw) in raw.certificates.unwrap_or_default() {
        let at = format!("{at}.certificates.\"{host}\"");
        let target = match HostPort::try_from(host) {
            Ok(target) => target,
            Err(e) => {
                errors.push(format!("{at}: {e}"));
                continue;
            }
        };
        if certificates.iter().any(|c| c.target == target) {
            errors.push(format!("{at}: \"{target}\" is listed twice"));
        }
        let names = AlertNames::new(
            format!("{id}/{target}"),
            format!("{name} certificate {target}"),
        );
        certificates.push(AppCertificate {
            alert: watched_alert(&at, raw, names, errors),
            target,
        });
    }

    let raw_checks = raw.health_checks.unwrap_or_default();
    if raw_checks.is_empty() && containers.is_empty() && certificates.is_empty() {
        errors.push(format!(
            "{at}: nothing to monitor, add healthChecks, containers or certificates"
        ));
    }
    let mut check_names = HashSet::new();
    let health_checks = raw_checks
        .into_iter()
        .enumerate()
        .map(|(i, raw_check)| {
            let at = format!("{at}.healthChecks[{i}]");
            let check = health_check(&at, &id, &name, raw_check, errors);
            if !check_names.insert(check.name.clone()) {
                errors.push(format!(
                    "{at}: another check is also named \"{}\", give one a \"name\"",
                    check.name
                ));
            }
            check
        })
        .collect();

    App {
        id,
        name,
        containers,
        health_checks,
        certificates,
    }
}

fn health_check(
    at: &str,
    app_id: &str,
    app_name: &str,
    raw: RawHealthCheck,
    errors: &mut Vec<String>,
) -> HealthCheck {
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

    let name = raw.name.unwrap_or_else(|| raw.url.to_string());
    let alert = raw.alert.map(|raw_alert| {
        let names = AlertNames::new(format!("{app_id}/{name}"), format!("{app_name} {name}"));
        state_alert(at, raw_alert, names, errors)
    });
    HealthCheck {
        name,
        url: raw.url,
        every,
        timeout,
        expect_code,
        content_type: raw.content_type,
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
                "apps": { "db": { "containers": { "db-1": {} } } }
            }"#,
        ))
        .unwrap();
        assert_eq!(ok.apps[0].containers[0].name, "db-1");

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "db": { "containers": { "db-1": {}, "": {} } } }
            }"#,
        ))
        .unwrap_err();
        // No docker section, empty name.
        assert_eq!(errors.len(), 2, "{errors:#?}");
    }

    #[test]
    fn checks_certificates() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "site": { "certificates": { "example.com": {}, "example.com:8443": {} } } }
            }"#,
        ))
        .unwrap();
        assert_eq!(config.apps[0].certificates.len(), 2);

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "site": { "certificates": {
                    "example.com": {}, "example.com:443": {}, "a/b": {}
                } } }
            }"#,
        ))
        .unwrap_err();
        // The same host twice (443 is the default), a path.
        assert_eq!(errors.len(), 2, "{errors:#?}");
    }

    #[test]
    fn rejects_bad_urls_while_parsing() {
        let result: Result<RawConfig, _> = serde_json::from_str(
            r#"{ "listen": "127.0.0.1:9100", "apps": { "a": { "healthChecks": [{ "url": "ftp://a/" }] } } }"#,
        );
        assert!(result.is_err());
    }
}
