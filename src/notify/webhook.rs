//! Sends the configured body, or a JSON one, and signs it if asked.

use std::fmt::Write;

use ring::hmac;
use serde_json::json;

use crate::config::{WEBHOOK_VARIABLES, Webhook, render_template};
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

fn body(config: &Webhook, context: &str, message: &Message) -> Result<String, Failure> {
    let status = if message.severity.is_some() {
        "firing"
    } else {
        "resolved"
    };
    let Some(template) = &config.body else {
        return Ok(json!({
            "context": context,
            "alert": message.alert,
            "status": status,
            "severity": message.severity.map(|severity| severity.to_string()),
            "title": message.title,
            "message": message.body,
        })
        .to_string());
    };
    let severity = message
        .severity
        .map(|severity| severity.to_string())
        .unwrap_or_default();
    let values = [
        context,
        &message.alert,
        status,
        &severity,
        &message.title,
        &message.body,
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

    fn message(severity: Option<Severity>) -> Message {
        Message {
            alert: "blog-db".to_owned(),
            title: "Blog \"db\"".to_owned(),
            body: "not running".to_owned(),
            severity,
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
        let Ok(body) = body(&webhook(None), "Box", &message(Some(Severity::Warning))) else {
            panic!();
        };
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            json,
            json!({
                "context": "Box",
                "alert": "blog-db",
                "status": "firing",
                "severity": "warning",
                "title": "Blog \"db\"",
                "message": "not running",
            })
        );
        let Ok(body) = super::body(&webhook(None), "Box", &message(None)) else {
            panic!();
        };
        assert!(body.contains(r#""severity":null"#) && body.contains(r#""status":"resolved""#));
    }

    #[test]
    fn fills_the_body_template() {
        let config = webhook(Some(
            r#"{"text": "[%{context}] %{title|json}: %{severity}"}"#,
        ));
        let Ok(body) = body(&config, "Box", &message(Some(Severity::Critical))) else {
            panic!();
        };
        assert_eq!(body, r#"{"text": "[Box] Blog \"db\": critical"}"#);
    }
}
