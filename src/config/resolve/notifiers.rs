use std::collections::BTreeMap;

use crate::config::raw::{RawHmac, RawNotifier, RawNtfy, RawWebhook};
use crate::config::resolved::{Hmac, Notifier, NotifierKind, Ntfy, WEBHOOK_VARIABLES, Webhook};
use crate::http::Url;

use super::headers::{header_name, headers};
use super::template::render;

/// Each one carries a body.
const WEBHOOK_METHODS: [&str; 3] = ["POST", "PUT", "PATCH"];
const DEFAULT_HMAC_HEADER: &str = "X-Signature";

pub(super) fn notifiers(
    raw: BTreeMap<String, RawNotifier>,
    context: &str,
    errors: &mut Vec<String>,
) -> Vec<Notifier> {
    raw.into_iter()
        .filter_map(|(name, raw_notifier)| notifier(name, raw_notifier, context, errors))
        .collect()
}

fn notifier(
    name: String,
    raw: RawNotifier,
    context: &str,
    errors: &mut Vec<String>,
) -> Option<Notifier> {
    let at = format!("notifiers.{name}");
    if name.is_empty() {
        errors.push("notifiers: a notifier name can't be empty".to_owned());
    }
    let kind = match raw {
        RawNotifier::Ntfy(raw) => NotifierKind::Ntfy(ntfy(&at, raw, errors)),
        RawNotifier::Webhook(raw) => NotifierKind::Webhook(webhook(&at, raw, context, errors)?),
    };
    Some(Notifier { name, kind })
}

fn ntfy(at: &str, raw: RawNtfy, errors: &mut Vec<String>) -> Ntfy {
    let topic_ok = (1..=64).contains(&raw.topic.len())
        && raw
            .topic
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !topic_ok {
        errors.push(format!(
            "{at}.topic: expected up to 64 letters, digits, \"-\" and \"_\""
        ));
    }
    if raw.token.as_deref().is_some_and(str::is_empty) {
        errors.push(format!("{at}.token: can't be empty, leave it out instead"));
    }
    if raw
        .address
        .as_ref()
        .is_some_and(|address| address.target.contains('?'))
    {
        errors.push(format!("{at}.address: can't have a query"));
    }
    Ntfy {
        address: raw.address,
        topic: raw.topic,
        token: raw.token,
    }
}

/// `url` and `headers` are filled in now, `body` per message.
fn webhook(at: &str, raw: RawWebhook, context: &str, errors: &mut Vec<String>) -> Option<Webhook> {
    let vars = [("context", context)];
    let method = raw
        .method
        .map_or("POST".to_owned(), |m| m.to_ascii_uppercase());
    if !WEBHOOK_METHODS.contains(&method.as_str()) {
        errors.push(format!(
            "{at}.method: \"{method}\" isn't one of {}",
            WEBHOOK_METHODS.join(", ")
        ));
    }
    let headers = headers(at, raw.headers.unwrap_or_default(), &vars, errors);
    if let Some(body) = &raw.body {
        let empty = WEBHOOK_VARIABLES.map(|name| (name, ""));
        if let Err(e) = render(body, &empty) {
            errors.push(format!("{at}.body: {e}"));
        }
    }
    let hmac = raw.hmac.map(|hmac| self::hmac(at, hmac, errors));
    let url = render(&raw.url, &vars)
        .and_then(Url::try_from)
        .map_err(|e| errors.push(format!("{at}.url: {e}")))
        .ok()?;
    Some(Webhook {
        url,
        method,
        headers,
        body: raw.body,
        hmac,
    })
}

fn hmac(at: &str, raw: RawHmac, errors: &mut Vec<String>) -> Hmac {
    if raw.secret.is_empty() {
        errors.push(format!("{at}.hmac.secret: can't be empty"));
    }
    let header = raw.header.unwrap_or_else(|| DEFAULT_HMAC_HEADER.to_owned());
    if let Err(e) = header_name(&header) {
        errors.push(format!("{at}.hmac.header: {e}"));
    }
    Hmac {
        secret: raw.secret,
        header,
    }
}

#[cfg(test)]
mod tests {
    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;
    use crate::config::resolved::NotifierKind;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn checks_notifiers() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "context": "Test box",
                "notifiers": { "phone": { "type": "ntfy", "topic": "test-topic_1", "token": "tk_x" } }
            }"#,
        ))
        .unwrap();
        assert_eq!(config.context, "Test box");
        let NotifierKind::Ntfy(ntfy) = &config.notifiers[0].kind else {
            panic!("{:?}", config.notifiers[0].kind);
        };
        assert_eq!(ntfy.address, None);
        assert_eq!(ntfy.topic, "test-topic_1");

        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "context": "",
                "notifiers": { "phone": { "type": "ntfy", "topic": "bad topic", "token": "", "address": "https://a/?b" } }
            }"#,
        ))
        .unwrap_err();
        // Empty context, bad topic, empty token, address with a query.
        assert_eq!(errors.len(), 4, "{errors:#?}");

        let result: Result<RawConfig, _> = serde_json::from_str(
            r#"{ "listen": "127.0.0.1:9100", "notifiers": { "a": { "type": "ntfy", "topic": "t", "tpoic": "t" } } }"#,
        );
        assert!(result.is_err(), "unknown ntfy fields are rejected");
    }

    #[test]
    fn resolves_a_webhook() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "context": "Box 1",
                "notifiers": {
                    "plain": { "type": "webhook", "url": "https://example.com/hook" },
                    "signed": {
                        "type": "webhook",
                        "url": "https://example.com/hook?from=%{context|url}",
                        "method": "put",
                        "headers": ["X-Token: abc"],
                        "body": "{\"text\": \"%{title|json}\"}",
                        "hmac": { "secret": "s3cr3t" }
                    }
                }
            }"#,
        ))
        .unwrap();
        let [plain, signed] = &config.notifiers[..] else {
            panic!("{:?}", config.notifiers);
        };
        let NotifierKind::Webhook(plain) = &plain.kind else {
            panic!("{plain:?}");
        };
        assert_eq!(plain.method, "POST");
        assert!(plain.body.is_none() && plain.hmac.is_none());
        let NotifierKind::Webhook(signed) = &signed.kind else {
            panic!("{signed:?}");
        };
        assert_eq!(signed.url.target, "/hook?from=Box%201");
        assert_eq!(signed.method, "PUT");
        assert_eq!(signed.headers, [("X-Token".to_owned(), "abc".to_owned())]);
        let hmac = signed.hmac.as_ref().unwrap();
        assert_eq!(
            (hmac.secret.as_str(), hmac.header.as_str()),
            ("s3cr3t", "X-Signature")
        );
    }

    #[test]
    fn checks_webhooks() {
        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "notifiers": { "hook": {
                    "type": "webhook",
                    "url": "ftp://example.com/",
                    "method": "GET",
                    "headers": ["Host: x"],
                    "body": "%{host}",
                    "hmac": { "secret": "", "header": "Bad Name" }
                } }
            }"#,
        ))
        .unwrap_err();
        // url, method, header, body variable, empty secret, header name.
        assert_eq!(errors.len(), 6, "{errors:#?}");
    }
}
