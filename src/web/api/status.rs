use axum::http::header;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::alerts::{self, Firing};

#[derive(Debug, PartialEq, Serialize)]
struct Status {
    alerts: Vec<FiringAlert>,
    /// The loudest alert's severity, or `"ok"` without any.
    level: String,
}

#[derive(Debug, PartialEq, Serialize)]
struct FiringAlert {
    id: String,
    label: String,
    severity: String,
    detail: String,
    /// When the problem started, in Unix seconds.
    since: u32,
}

/// `GET /status`: the firing alerts, for scripts and other monitors.
pub async fn status() -> impl IntoResponse {
    let body = serde_json::to_string(&summarize(&alerts::firing()))
        .expect("serializing plain strings and numbers can't fail");
    ([(header::CONTENT_TYPE, "application/json")], body)
}

fn summarize(firing: &[Firing]) -> Status {
    let loudest = firing
        .iter()
        .map(|f| f.severity)
        .reduce(|a, b| if b.is_louder_than(a) { b } else { a });
    Status {
        alerts: firing
            .iter()
            .map(|f| FiringAlert {
                id: f.alert.id.clone(),
                label: f.alert.label.clone(),
                severity: f.severity.to_string(),
                detail: f.detail.clone(),
                since: f.since,
            })
            .collect(),
        level: loudest.map_or("ok".to_owned(), |s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::{Alert, Severity};

    fn firing(id: &str, severity: Severity) -> Firing {
        Firing {
            alert: Box::leak(Box::new(Alert {
                id: id.to_owned(),
                label: format!("{id} label"),
                severity,
                after: Duration::ZERO,
                parents: Vec::new(),
                threshold: None,
                restarts: None,
            })),
            since: 100,
            detail: "down".to_owned(),
            severity,
        }
    }

    #[test]
    fn level_is_the_loudest_severity() {
        assert_eq!(summarize(&[]).level, "ok");
        let status = summarize(&[
            firing("a", Severity::Info),
            firing("b", Severity::Critical),
            firing("c", Severity::Warning),
        ]);
        assert_eq!(status.level, "critical");
        assert_eq!(
            status.alerts[1],
            FiringAlert {
                id: "b".to_owned(),
                label: "b label".to_owned(),
                severity: "critical".to_owned(),
                detail: "down".to_owned(),
                since: 100,
            }
        );
    }
}
