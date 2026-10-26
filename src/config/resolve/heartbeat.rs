use crate::config::raw::RawHeartbeat;
use crate::config::resolved::Heartbeat;
use crate::http::Url;

use super::every::{DEFAULT_EVERY, every};
use super::headers::headers;
use super::template::render;

const METHODS: [&str; 5] = ["GET", "HEAD", "POST", "PUT", "PATCH"];

/// `context` is the only template variable.
pub(super) fn heartbeat(
    raw: RawHeartbeat,
    context: &str,
    errors: &mut Vec<String>,
) -> Option<Heartbeat> {
    let vars = [("context", context)];
    let every = every("heartbeat", raw.every, DEFAULT_EVERY, errors);

    let method = raw
        .method
        .map_or("GET".to_owned(), |m| m.to_ascii_uppercase());
    if !METHODS.contains(&method.as_str()) {
        errors.push(format!(
            "heartbeat.method: \"{method}\" isn't one of {}",
            METHODS.join(", ")
        ));
    }

    let headers = headers("heartbeat", raw.headers.unwrap_or_default(), &vars, errors);

    let body = raw.body.and_then(|body| {
        render(&body, &vars)
            .map_err(|e| errors.push(format!("heartbeat.body: {e}")))
            .ok()
    });
    if body.is_some() && matches!(method.as_str(), "GET" | "HEAD") {
        errors.push(format!(
            "heartbeat.body: a {method} request has no body, use POST or PUT"
        ));
    }

    let url = render(&raw.url, &vars)
        .and_then(Url::try_from)
        .map_err(|e| errors.push(format!("heartbeat.url: {e}")))
        .ok()?;
    Some(Heartbeat {
        url,
        method,
        headers,
        body,
        every,
    })
}

#[cfg(test)]
mod tests {
    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn resolves_a_heartbeat() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "context": "Box 1",
                "heartbeat": {
                    "url": "https://example.com/ping/abc?m=%{context|url}",
                    "method": "post",
                    "headers": ["X-Context: %{context}", "content-type: application/json"],
                    "body": "{ \"c\": \"%{context|json}\" }"
                }
            }"#,
        ))
        .unwrap();
        let heartbeat = config.heartbeat.unwrap();
        assert_eq!(heartbeat.url.target, "/ping/abc?m=Box%201");
        assert_eq!(heartbeat.method, "POST");
        assert_eq!(
            heartbeat.headers,
            [
                ("X-Context".to_owned(), "Box 1".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned())
            ]
        );
        assert_eq!(heartbeat.body.as_deref(), Some(r#"{ "c": "Box 1" }"#));
        assert_eq!(heartbeat.every.as_secs(), 60);
    }

    #[test]
    fn checks_a_heartbeat() {
        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "heartbeat": {
                    "url": "https://example.com/%{host}",
                    "method": "DELETE",
                    "headers": ["no colon", "Host: x", "Bad Name: x"],
                    "body": "%{context|html}",
                    "every": "0s"
                }
            }"#,
        ))
        .unwrap_err();
        // every, method, 3 headers, body filter, url variable.
        assert_eq!(errors.len(), 7, "{errors:#?}");

        let errors = resolve(parse(
            r#"{ "listen": "127.0.0.1:9100", "heartbeat": { "url": "https://example.com/", "body": "x" } }"#,
        ))
        .unwrap_err();
        assert_eq!(errors.len(), 1, "a GET with a body: {errors:#?}");
    }
}
