use std::fmt::Write;

/// Replaces `%{name}` with a variable's value as it is, `%{name|json}` with
/// it escaped for inside a JSON string, and `%{name|url}` with it
/// percent-encoded. Anything else is copied, so `%20` stays `%20`.
pub(crate) fn render(text: &str, vars: &[(&str, &str)]) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("%{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find('}')
            .ok_or_else(|| format!("\"%{{\" without a closing \"}}\" in \"{text}\""))?;
        let placeholder = &after[..end];
        out.push_str(&substitute(placeholder, vars)?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn substitute(placeholder: &str, vars: &[(&str, &str)]) -> Result<String, String> {
    let (name, filter) = match placeholder.split_once('|') {
        Some((name, filter)) => (name, Some(filter)),
        None => (placeholder, None),
    };
    let known = || {
        vars.iter()
            .map(|(name, _)| format!("%{{{name}}}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let value = vars
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| format!("unknown variable %{{{placeholder}}}, there's {}", known()))?;
    match filter {
        None => Ok(value.to_owned()),
        Some("json") => Ok(json_escape(value)),
        Some("url") => Ok(percent_encode(value)),
        Some(other) => Err(format!(
            "%{{{placeholder}}}: unknown filter \"{other}\", expected json or url"
        )),
    }
}

/// What goes between the quotes of a JSON string.
fn json_escape(value: &str) -> String {
    let quoted = serde_json::Value::from(value).to_string();
    quoted[1..quoted.len() - 1].to_owned()
}

/// Keeps only the characters URLs never need to encode.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARS: [(&str, &str); 1] = [("context", "Box \"1\"/ü")];

    #[test]
    fn substitutes_and_escapes() {
        let render = |text| render(text, &VARS).unwrap();
        assert_eq!(render("[%{context}]"), "[Box \"1\"/ü]");
        assert_eq!(
            render(r#"{"c":"%{context|json}"}"#),
            r#"{"c":"Box \"1\"/ü"}"#
        );
        assert_eq!(
            render("?m=%{context|url}&x=%20"),
            "?m=Box%20%221%22%2F%C3%BC&x=%20"
        );
        assert_eq!(render("no placeholders"), "no placeholders");
    }

    #[test]
    fn rejects_bad_placeholders() {
        for text in ["%{host}", "%{context|html}", "%{context"] {
            assert!(render(text, &VARS).is_err(), "{text}");
        }
    }
}
