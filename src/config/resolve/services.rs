use std::collections::BTreeMap;

use crate::config::raw::{RawAlert, RawRestarts, RawService, RawServiceAlert, RawServices};
use crate::config::resolved::{Alert, AppService, RestartLimit, ServicesConfig};
use crate::config::types::Dur;

use super::alerts::{AlertNames, state_alert};
use super::every::{DEFAULT_EVERY, every};

pub(super) fn services_config(raw: RawServices, errors: &mut Vec<String>) -> ServicesConfig {
    ServicesConfig {
        every: every("services", raw.every, DEFAULT_EVERY, errors),
    }
}

/// `app_at` is the app's config path, e.g. `apps.web`.
pub(super) fn app_services(
    app_at: &str,
    app_id: &str,
    app_name: &str,
    raw: BTreeMap<String, RawService>,
    config: &ServicesConfig,
    errors: &mut Vec<String>,
) -> Vec<AppService> {
    raw.into_iter()
        .map(|(name, raw)| {
            let at = format!("{app_at}.services.\"{name}\"");
            let names = AlertNames::new(
                format!("{app_id}/{name}"),
                format!("{app_name} service {name}"),
            );
            app_service(&at, name, raw, names, config, errors)
        })
        .collect()
}

fn app_service(
    at: &str,
    name: String,
    raw: RawService,
    names: AlertNames,
    config: &ServicesConfig,
    errors: &mut Vec<String>,
) -> AppService {
    if name.is_empty() || name.starts_with('-') || name.contains(char::is_whitespace) {
        errors.push(format!(
            "{at}: not a service name, it can't be empty, start with \"-\" or have spaces"
        ));
    }
    let alert = raw
        .alert
        .map(|raw_alert| service_alert(at, raw_alert, names, config, errors));
    AppService { name, alert }
}

/// A state alert that also fires when the service restarts too often.
fn service_alert(
    at: &str,
    raw: RawServiceAlert,
    names: AlertNames,
    config: &ServicesConfig,
    errors: &mut Vec<String>,
) -> Alert {
    let restarts = raw
        .restarts
        .map(|limit| restart_limit(at, limit, config, errors));
    let state = RawAlert {
        duration: raw.duration,
        severity: raw.severity,
        id: raw.id,
        depends_on: raw.depends_on,
    };
    Alert {
        restarts,
        ..state_alert(at, state, names, errors)
    }
}

fn restart_limit(
    at: &str,
    raw: RawRestarts,
    config: &ServicesConfig,
    errors: &mut Vec<String>,
) -> RestartLimit {
    if cfg!(target_os = "freebsd") {
        errors.push(format!(
            "{at}.alert.restarts: not supported on FreeBSD, rc.d doesn't restart services"
        ));
    }
    let within = raw.within.get();
    if within < config.every {
        errors.push(format!(
            "{at}.alert.restarts.within ({}) is shorter than services.every ({}), it would only count one check",
            raw.within,
            Dur::from(config.every),
        ));
    }
    RestartLimit {
        above: raw.above,
        within,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    #[cfg(not(target_os = "freebsd"))]
    fn resolves_services() {
        use std::time::Duration;

        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "services": { "every": "30s" },
                "apps": {
                    "web": {
                        "name": "Web",
                        "services": {
                            "web-api": { "alert": { "for": "1m", "restarts": { "above": 3, "within": "1h" } } },
                            "web-cron": {}
                        }
                    }
                }
            }"#,
        ))
        .unwrap();
        assert_eq!(config.services.every, Duration::from_secs(30));
        let [api, cron] = &config.apps[0].services[..] else {
            panic!("{:#?}", config.apps[0].services);
        };
        let alert = api.alert.as_ref().unwrap();
        assert_eq!(
            (alert.id.as_str(), alert.label.as_str()),
            ("web/web-api", "Web service web-api")
        );
        assert_eq!(alert.after, Duration::from_secs(60));
        let limit = alert.restarts.unwrap();
        assert_eq!((limit.above, limit.within), (3, Duration::from_secs(3600)));
        assert!(cron.alert.is_none());
    }

    #[test]
    fn checks_services() {
        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "services": { "every": "1m" },
                "apps": {
                    "web": {
                        "services": {
                            "": {},
                            "-x": {},
                            "a b": {},
                            "ok": { "alert": { "restarts": { "above": 1, "within": "30s" } } }
                        }
                    }
                }
            }"#,
        ))
        .unwrap_err();
        // Three bad names, `within` shorter than `every`, and on FreeBSD
        // `restarts` at all.
        let expected = if cfg!(target_os = "freebsd") { 5 } else { 4 };
        assert_eq!(errors.len(), expected, "{errors:#?}");

        let result: Result<RawConfig, _> = serde_json::from_str(
            r#"{ "listen": "127.0.0.1:9100", "apps": { "a": { "services": { "s": { "alert": { "restarts": { "above": 1 } } } } } } }"#,
        );
        assert!(result.is_err(), "`within` is required");
    }
}
