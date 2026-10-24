use std::collections::BTreeMap;

use crate::config::raw::RawJob;
use crate::config::resolved::AppJob;
use crate::config::types::Dur;
use crate::cron::Schedule;
use crate::model::now;

use super::alerts::{AlertNames, state_alert};

/// Long enough that guessing one isn't practical.
const MIN_TOKEN_LENGTH: usize = 16;

/// `app_at` is the app's config path, e.g. `apps.web`.
pub(super) fn app_jobs(
    app_at: &str,
    app_id: &str,
    app_name: &str,
    raw: BTreeMap<String, RawJob>,
    errors: &mut Vec<String>,
) -> Vec<AppJob> {
    if !raw.is_empty() && !url_safe(app_id) {
        errors.push(format!(
            "{app_at}: an app with jobs needs an ID of letters, digits, \".\", \"_\" and \"-\", it's part of the jobs' URLs"
        ));
    }
    raw.into_iter()
        .filter_map(|(name, raw)| {
            let at = format!("{app_at}.jobs.\"{name}\"");
            let names =
                AlertNames::new(format!("{app_id}/{name}"), format!("{app_name} job {name}"));
            app_job(&at, name, raw, names, errors)
        })
        .collect()
}

fn app_job(
    at: &str,
    name: String,
    raw: RawJob,
    names: AlertNames,
    errors: &mut Vec<String>,
) -> Option<AppJob> {
    if !url_safe(&name) {
        errors.push(format!(
            "{at}: a job name is letters, digits, \".\", \"_\" and \"-\", it's part of the job's URLs"
        ));
    }
    if raw.token.chars().count() < MIN_TOKEN_LENGTH || raw.token.contains(char::is_whitespace) {
        errors.push(format!(
            "{at}.token: must be at least {MIN_TOKEN_LENGTH} characters, without spaces"
        ));
    }
    if raw.max_duration.is_some_and(|max| max.get().is_zero()) {
        errors.push(format!("{at}.maxDuration: must be more than 0"));
    }
    let schedule = match Schedule::parse(&raw.schedule) {
        Ok(schedule) if schedule.next_after(now()).is_none() => {
            errors.push(format!("{at}.schedule: \"{schedule}\" is never due"));
            return None;
        }
        Ok(schedule) => schedule,
        Err(e) => {
            errors.push(format!("{at}.schedule: {e}"));
            return None;
        }
    };
    let alert = raw
        .alert
        .map(|raw_alert| state_alert(at, raw_alert, names, errors));
    Some(AppJob {
        name,
        token: raw.token,
        schedule,
        max_duration: raw.max_duration.map(Dur::get),
        alert,
    })
}

fn url_safe(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn resolves_jobs() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": {
                    "backup": {
                        "name": "Backup",
                        "jobs": {
                            "nightly": {
                                "token": "0123456789abcdef",
                                "schedule": "0 3 * * *",
                                "maxDuration": "6h",
                                "alert": { "for": "20m", "severity": "critical" }
                            }
                        }
                    }
                }
            }"#,
        ))
        .unwrap();
        let job = &config.apps[0].jobs[0];
        assert_eq!(job.schedule.to_string(), "0 3 * * *");
        assert_eq!(job.max_duration, Some(Duration::from_secs(6 * 3600)));
        let alert = job.alert.as_ref().unwrap();
        assert_eq!(
            (alert.id.as_str(), alert.label.as_str()),
            ("backup/nightly", "Backup job nightly")
        );
        assert_eq!(alert.after, Duration::from_secs(20 * 60));
    }

    #[test]
    fn checks_jobs() {
        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": {
                    "my app": {
                        "jobs": {
                            "a/b": { "token": "0123456789abcdef", "schedule": "@daily" },
                            "short": { "token": "abc", "schedule": "@daily", "maxDuration": "0s" },
                            "bad": { "token": "0123456789abcdef", "schedule": "0 3 * *" },
                            "never": { "token": "0123456789abcdef", "schedule": "0 0 31 2 *" }
                        }
                    }
                }
            }"#,
        ))
        .unwrap_err();
        // App ID, job name, short token, zero maxDuration, 4 fields, never due.
        assert_eq!(errors.len(), 6, "{errors:#?}");

        let missing: Result<RawConfig, _> = serde_json::from_str(
            r#"{ "listen": "127.0.0.1:9100", "apps": { "a": { "jobs": { "j": { "token": "0123456789abcdef" } } } } }"#,
        );
        assert!(missing.is_err(), "`schedule` is required");
    }
}
