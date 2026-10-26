//! Publishes as JSON to the server's root, which keeps non-ASCII titles out
//! of HTTP headers.

use std::sync::LazyLock;

use serde_json::json;

use crate::config::{Ntfy, Severity};
use crate::http::{Url, describe, post_json};

use super::message::Message;
use super::outbox::{Failure, judge};

static DEFAULT_ADDRESS: LazyLock<Url> = LazyLock::new(|| {
    Url::try_from("https://ntfy.sh".to_owned()).expect("the default address is a valid URL")
});

pub(super) async fn publish(
    config: &Ntfy,
    context: &str,
    message: &Message,
) -> Result<(), Failure> {
    // Priority 2 shows the notification without a sound or vibration.
    let (priority, tag) = match (message.ended, message.severity) {
        (Some(_), _) => (2, "white_check_mark"),
        (None, Severity::Critical) => (5, "rotating_light"),
        (None, Severity::Warning) => (4, "warning"),
        (None, Severity::Info) => (3, "information_source"),
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
    let response = post_json(address, &headers, body.as_bytes())
        .await
        .map_err(|e| Failure::Retry(describe(&e)))?;
    judge(&response)
}
