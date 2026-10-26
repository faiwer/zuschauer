use crate::http::RESERVED_HEADERS;

use super::template::render;

/// Characters allowed in a header name, besides letters and digits.
const NAME_SYMBOLS: &str = "!#$%&'*+-.^_`|~";

/// `raw`: `name: value` templates, at `{at}.headers`.
pub(super) fn headers(
    at: &str,
    raw: Vec<String>,
    vars: &[(&str, &str)],
    errors: &mut Vec<String>,
) -> Vec<(String, String)> {
    raw.iter()
        .enumerate()
        .filter_map(|(i, raw_header)| {
            render(raw_header, vars)
                .and_then(|text| header(&text))
                .map_err(|e| errors.push(format!("{at}.headers[{i}]: {e}")))
                .ok()
        })
        .collect()
}

/// A header name zuschauer can send: a token, and not one it sets itself.
pub(super) fn header_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || NAME_SYMBOLS.contains(c));
    if !valid {
        return Err(format!("\"{name}\" isn't a valid header name"));
    }
    if RESERVED_HEADERS
        .iter()
        .any(|r| r.eq_ignore_ascii_case(name))
    {
        return Err(format!("{name} is set by zuschauer"));
    }
    Ok(())
}

/// `name: value`.
fn header(text: &str) -> Result<(String, String), String> {
    let (name, value) = text
        .split_once(':')
        .ok_or_else(|| format!("\"{text}\" isn't \"name: value\""))?;
    let (name, value) = (name.trim(), value.trim());
    header_name(name)?;
    if value.chars().any(|c| c.is_control() && c != '\t') {
        return Err(format!(
            "{name}: the value can't have line breaks or control characters"
        ));
    }
    Ok((name.to_owned(), value.to_owned()))
}
