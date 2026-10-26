use std::time::Duration;

use crate::config::{Alert, Dur, Severity};
use crate::model::Timestamp;
use crate::notify::Message;

pub(super) fn fired(
    alert: &Alert,
    severity: Severity,
    detail: &str,
    since: Timestamp,
    at: Timestamp,
) -> Message {
    let lasted = at.saturating_sub(since);
    let body = if lasted == 0 {
        detail.to_owned()
    } else {
        format!("{detail}, for {}", span(lasted))
    };
    Message {
        alert: alert.id.clone(),
        label: alert.label.clone(),
        title: alert.label.clone(),
        body,
        detail: detail.to_owned(),
        severity,
        since,
        ended: None,
    }
}

pub(super) fn resolved(
    alert: &Alert,
    severity: Severity,
    detail: String,
    since: Timestamp,
    at: Timestamp,
) -> Message {
    Message {
        alert: alert.id.clone(),
        label: alert.label.clone(),
        title: format!("{} resolved", alert.label),
        body: format!("after {}", span(at.saturating_sub(since))),
        detail,
        severity,
        since,
        ended: Some(at),
    }
}

fn span(seconds: Timestamp) -> Dur {
    Dur::from(Duration::from_secs(seconds.into()))
}
