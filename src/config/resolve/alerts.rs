use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use crate::config::raw::{RawAlert, RawThresholdAlert, RawWatched};
use crate::config::resolved::{Alert, Config, Threshold};
use crate::config::types::Dur;

/// An alert's default ID and its label in notifications, both derived from
/// what it watches.
pub(super) struct AlertNames {
    id: String,
    label: String,
}

impl AlertNames {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// `every` is how often the value is read.
pub(super) fn threshold_alert<T: Copy + Into<f32> + fmt::Display>(
    at: &str,
    raw: RawThresholdAlert<T>,
    every: Duration,
    unit: &'static str,
    names: AlertNames,
    errors: &mut Vec<String>,
) -> Alert {
    if raw.duration.get() < every {
        errors.push(format!(
            "{at}.alert.for ({}) is shorter than {at}.every ({}), it would fire on a single sample",
            raw.duration,
            Dur::from(every),
        ));
    }
    if let Some(below) = raw.resolve_below
        && below.into() > raw.above.into()
    {
        errors.push(format!(
            "{at}.alert.resolveBelow ({below}) is above {at}.alert.above ({})",
            raw.above
        ));
    }
    Alert {
        id: alert_id(at, raw.id, names.id, errors),
        label: names.label,
        severity: raw.severity.unwrap_or_default(),
        after: raw.duration.get(),
        parents: raw.depends_on.unwrap_or_default(),
        threshold: Some(Threshold {
            above: raw.above.into(),
            resolve_below: raw.resolve_below.map(Into::into),
            unit,
        }),
    }
}

pub(super) fn watched_alert(
    at: &str,
    raw: RawWatched,
    names: AlertNames,
    errors: &mut Vec<String>,
) -> Option<Alert> {
    raw.alert
        .map(|raw_alert| state_alert(at, raw_alert, names, errors))
}

pub(super) fn state_alert(
    at: &str,
    raw: RawAlert,
    names: AlertNames,
    errors: &mut Vec<String>,
) -> Alert {
    Alert {
        id: alert_id(at, raw.id, names.id, errors),
        label: names.label,
        severity: raw.severity.unwrap_or_default(),
        after: raw.duration.map_or(Duration::ZERO, Dur::get),
        parents: raw.depends_on.unwrap_or_default(),
        threshold: None,
    }
}

fn alert_id(at: &str, id: Option<String>, default: String, errors: &mut Vec<String>) -> String {
    if id.as_deref().is_some_and(str::is_empty) {
        errors.push(format!(
            "{at}.alert.id: can't be empty, leave it out instead"
        ));
    }
    id.unwrap_or(default)
}

/// Checks that alert IDs are unique and that `dependsOn` lists existing IDs
/// without cycles, then replaces each alert's direct parents with all of its
/// ancestors.
pub(super) fn link_alerts(config: &mut Config, errors: &mut Vec<String>) {
    let mut direct: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for alert in config.alerts() {
        if direct
            .insert(alert.id.clone(), alert.parents.clone())
            .is_some()
        {
            errors.push(format!(
                "alert ID \"{}\" is used twice, give one an \"id\"",
                alert.id
            ));
        }
    }

    let mut ancestors = Vec::new();
    for alert in config.alerts() {
        for parent in &alert.parents {
            if !direct.contains_key(parent) {
                errors.push(format!(
                    "alert \"{}\": dependsOn \"{parent}\" is not an alert ID",
                    alert.id
                ));
            }
        }
        let mut seen: Vec<String> = Vec::new();
        let mut queue: Vec<&str> = alert.parents.iter().map(String::as_str).collect();
        while let Some(id) = queue.pop() {
            if id == alert.id {
                errors.push(format!("alert \"{id}\" depends on itself"));
                break;
            }
            if seen.iter().any(|s| s == id) {
                continue;
            }
            seen.push(id.to_owned());
            if let Some(parents) = direct.get(id) {
                queue.extend(parents.iter().map(String::as_str));
            }
        }
        ancestors.push(seen);
    }

    for (alert, ancestors) in config.alerts_mut().into_iter().zip(ancestors) {
        alert.parents = ancestors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;
    use crate::config::types::Severity;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn names_alerts_and_links_dependencies() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "docker": {},
                "host": {
                    "cpu": { "alert": { "above": "85%", "for": "10m", "resolveBelow": "70%" } }
                },
                "apps": {
                    "web": {
                        "name": "Web",
                        "containers": { "db": { "alert": { "id": "db", "severity": "critical" } } },
                        "healthChecks": [
                            { "name": "api", "url": "http://a/", "alert": { "for": "1m", "dependsOn": ["web/proxy"] } },
                            { "name": "proxy", "url": "http://b/", "alert": { "dependsOn": ["db"] } }
                        ]
                    }
                }
            }"#,
        ))
        .unwrap();
        let alerts: Vec<_> = config.alerts().collect();
        let ids: Vec<_> = alerts.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["cpu", "web/api", "web/proxy", "db"]);

        let cpu = alerts[0];
        assert_eq!(cpu.severity, Severity::Warning);
        let threshold = cpu.threshold.as_ref().unwrap();
        assert_eq!(
            (threshold.above, threshold.resolve_below),
            (85.0, Some(70.0))
        );

        let api = alerts[1];
        assert_eq!(api.label, "Web api");
        assert_eq!(api.after, Duration::from_secs(60));
        assert_eq!(api.parents, ["web/proxy", "db"]);

        let db = alerts[3];
        assert_eq!(
            (db.severity, db.after),
            (Severity::Critical, Duration::ZERO)
        );
        assert_eq!(db.label, "Web container db");
    }

    #[test]
    fn collects_alert_errors() {
        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "host": {
                    "cpu": { "alert": { "above": "85%", "for": "10m", "resolveBelow": "90%", "id": "" } },
                    "memory": { "alert": { "above": "85%", "for": "10m", "id": "x" } }
                },
                "apps": {
                    "web": {
                        "healthChecks": [
                            { "name": "a", "url": "http://a/", "alert": { "dependsOn": ["b", "nope"] } },
                            { "name": "b", "url": "http://b/", "alert": { "id": "x", "dependsOn": ["web/a"] } }
                        ]
                    }
                }
            }"#,
        ))
        .unwrap_err();
        // resolveBelow > above, empty ID, "x" twice, and two unknown
        // dependsOn IDs: "nope", and "b", whose alert is "x".
        assert_eq!(errors.len(), 5, "{errors:#?}");

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": {
                    "web": {
                        "healthChecks": [
                            { "name": "a", "url": "http://a/", "alert": { "dependsOn": ["web/b"] } },
                            { "name": "b", "url": "http://b/", "alert": { "dependsOn": ["web/a"] } }
                        ]
                    }
                }
            }"#,
        ))
        .unwrap_err();
        // Each alert of the cycle depends on itself.
        assert_eq!(errors.len(), 2, "{errors:#?}");
    }
}
