use std::collections::BTreeMap;

use crate::config::raw::{RawNotifier, RawNtfy};
use crate::config::resolved::{Notifier, NotifierKind, Ntfy};

pub(super) fn notifiers(
    raw: BTreeMap<String, RawNotifier>,
    errors: &mut Vec<String>,
) -> Vec<Notifier> {
    raw.into_iter()
        .map(|(name, raw_notifier)| notifier(name, raw_notifier, errors))
        .collect()
}

fn notifier(name: String, raw: RawNotifier, errors: &mut Vec<String>) -> Notifier {
    let at = format!("notifiers.{name}");
    if name.is_empty() {
        errors.push("notifiers: a notifier name can't be empty".to_owned());
    }
    let kind = match raw {
        RawNotifier::Ntfy(raw) => NotifierKind::Ntfy(ntfy(&at, raw, errors)),
    };
    Notifier { name, kind }
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
        let NotifierKind::Ntfy(ntfy) = &config.notifiers[0].kind;
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
}
