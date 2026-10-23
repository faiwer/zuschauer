//! Publishes as JSON to the server's root, which keeps non-ASCII titles out
//! of HTTP headers.

use std::sync::LazyLock;

use serde_json::json;

use crate::config::{Ntfy, Severity};
use crate::http::{Url, describe, post};

use super::message::Message;
use super::outbox::Failure;

static DEFAULT_ADDRESS: LazyLock<Url> = LazyLock::new(|| {
    Url::try_from("https://ntfy.sh".to_owned()).expect("the default address is a valid URL")
});

pub(super) async fn publish(
    config: &Ntfy,
    context: &str,
    message: &Message,
) -> Result<(), Failure> {
    // Priority 2 shows the notification without a sound or vibration.
    let (priority, tag) = match message.severity {
        Some(Severity::Critical) => (5, "rotating_light"),
        Some(Severity::Warning) => (4, "warning"),
        Some(Severity::Info) => (3, "information_source"),
        None => (2, "white_check_mark"),
    };
    let body = json!({
        "topic": config.topic,
        "title": format!("[{context}] {}", message.title),
        "message": message.body,
        "priority": priority,
        "tags": [tag],
    })
    .to_string();
    let authorization = config.token.as_ref().map(|token| format!("Bearer {token}"));
    let headers: Vec<_> = authorization
        .iter()
        .map(|value| ("Authorization", value.as_str()))
        .collect();

    let address = config.address.as_ref().unwrap_or(&DEFAULT_ADDRESS);
    let response = post(address, &headers, body.as_bytes())
        .await
        .map_err(|e| Failure::Retry(describe(&e)))?;
    let status = response.head.status;
    if (200..300).contains(&status) {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&response.body[..response.body.len().min(200)]);
    let reason = format!("HTTP {status}: {}", text.trim());
    if status == 429 || status >= 500 {
        Err(Failure::Retry(reason))
    } else {
        Err(Failure::Rejected(reason))
    }
}
