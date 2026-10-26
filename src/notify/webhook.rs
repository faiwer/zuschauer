//! Sends the configured body, or a JSON one, and signs it if asked.

use std::fmt::Write;

use ring::hmac;
use serde_json::json;

use crate::config::{WEBHOOK_VARIABLES, Webhook, render_template};
use crate::cron::format_utc;
use crate::http::{describe, request};

use super::message::Message;
use super::outbox::{Failure, judge};

pub(super) async fn publish(
    config: &Webhook,
    context: &str,
    message: &Message,
) -> Result<(), Failure> {
    let body = body(config, context, message)?;
    let signature = config
        .hmac
        .as_ref()
        .map(|hmac| (hmac.header.as_str(), sign(&hmac.secret, body.as_bytes())));

    let mut headers: Vec<(&str, &str)> = config
        .headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    {
        headers.push(("Content-Type", "application/json"));
    }
    if let Some((name, value)) = &signature {
        headers.push((name, value));
    }

    let response = request(&config.url, &config.method, &headers, Some(body.as_bytes()))
        .await
        .map_err(|e| Failure::Retry(describe(&e)))?;
    judge(&response)
}

/// What Alertmanager sends as `endsAt` while an alert fires.
const NOT_ENDED: &str = "0001-01-01T00:00:00Z";

fn body(config: &Webhook, context: &str, message: &Message) -> Result<String, Failure> {
    let status = if message.ended.is_some() {
        "resolved"
    } else {
        "firing"
    };
    let severity = message.severity.to_string();
    let since = format_utc(message.since);
    let ended = message.ended.map(format_utc);
    let Some(template) = &config.body else {
        return Ok(json!({
            "context": context,
            "alert": message.alert,
            "label": message.label,
            "status": status,
            "severity": severity,
            "title": message.title,
            "message": message.body,
            "detail": message.detail,
            "since": since,
            "ended": ended,
        })
        .to_string());
    };
    let values = [
        context,
        &message.alert,
        &message.label,
        status,
        &severity,
        &message.title,
        &message.body,
        &message.detail,
        &since,
        ended.as_deref().unwrap_or(NOT_ENDED),
    ];
    let vars: Vec<(&str, &str)> = WEBHOOK_VARIABLES.into_iter().zip(values).collect();
    render_template(template, &vars).map_err(Failure::Rejected)
}

/// `sha256=<hex>`, as GitHub sends it.
fn sign(secret: &str, body: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, body);
    let mut signature = "sha256=".to_owned();
    for byte in tag.as_ref() {
        let _ = write!(signature, "{byte:02x}");
    }
    signature
}

#[cfg(test)]
mod tests {
    use crate::config::Severity;
    use crate::http::Url;
    use crate::model::Timestamp;

    use super::*;

    fn webhook(body: Option<&str>) -> Webhook {
        Webhook {
            url: Url::try_from("https://example.com/hook".to_owned()).unwrap(),
            method: "POST".to_owned(),
            headers: Vec::new(),
            body: body.map(str::to_owned),
            hmac: None,
        }
    }

    fn message(ended: Option<Timestamp>) -> Message {
        Message {
            alert: "blog-db".to_owned(),
            label: "Blog \"db\"".to_owned(),
            title: "Blog \"db\"".to_owned(),
            body: "not running, for 1m".to_owned(),
            detail: "not running".to_owned(),
            severity: Severity::Warning,
            since: 1_790_000_000,
            ended,
        }
    }

    #[test]
    fn signs_like_rfc_4231() {
        // RFC 4231, test case 2.
        assert_eq!(
            sign("Jefe", b"what do ya want for nothing?"),
            "sha256=5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn sends_the_default_json() {
        let Ok(body) = body(&webhook(None), "Box", &message(None)) else {
            panic!();
        };
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            json,
            json!({
                "context": "Box",
                "alert": "blog-db",
                "label": "Blog \"db\"",
                "status": "firing",
                "severity": "warning",
                "title": "Blog \"db\"",
                "message": "not running, for 1m",
                "detail": "not running",
                "since": "2026-09-21T14:13:20Z",
                "ended": null,
            })
        );
        let Ok(body) = super::body(&webhook(None), "Box", &message(Some(1_790_000_060))) else {
            panic!();
        };
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["status"], "resolved");
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["ended"], "2026-09-21T14:14:20Z");
    }

    #[test]
    fn fills_the_body_template() {
        let config = webhook(Some(
            r#"{"text": "[%{context}] %{label|json}: %{detail}", "endsAt": "%{ended}"}"#,
        ));
        let Ok(firing) = body(&config, "Box", &message(None)) else {
            panic!();
        };
        assert_eq!(
            firing,
            r#"{"text": "[Box] Blog \"db\": not running", "endsAt": "0001-01-01T00:00:00Z"}"#
        );
        let Ok(resolved) = body(&config, "Box", &message(Some(1_790_000_060))) else {
            panic!();
        };
        assert!(
            resolved.ends_with(r#""endsAt": "2026-09-21T14:14:20Z"}"#),
            "{resolved}"
        );
    }
}
