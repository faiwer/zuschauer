use crate::config::HealthCheck;
use crate::http::Head;

/// `Err` says why the response doesn't count as healthy.
pub(super) fn verdict(check: &HealthCheck, head: &Head) -> Result<(), String> {
    if head.status != check.expect_code {
        return Err(format!(
            "got {}, expected {}",
            head.status, check.expect_code
        ));
    }

    if let Some(wanted) = &check.content_type {
        let got = head.content_type.as_deref().unwrap_or("");
        if !got
            .to_ascii_lowercase()
            .contains(&wanted.to_ascii_lowercase())
        {
            let got = if got.is_empty() { "none" } else { got };
            return Err(format!("content type {got}, expected {wanted}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn check(expect_code: u16, content_type: Option<&str>) -> HealthCheck {
        HealthCheck {
            name: "test".to_owned(),
            url: "http://127.0.0.1/".to_owned().try_into().unwrap(),
            every: Duration::from_secs(60),
            timeout: Duration::from_secs(10),
            expect_code,
            content_type: content_type.map(str::to_owned),
            alert: None,
        }
    }

    fn head(status: u16, content_type: Option<&str>) -> Head {
        Head::new(status, content_type.map(str::to_owned))
    }

    #[test]
    fn accepts_matching_responses() {
        assert_eq!(verdict(&check(200, None), &head(200, None)), Ok(()));
        assert_eq!(
            verdict(
                &check(200, Some("application/json")),
                &head(200, Some("Application/JSON; charset=utf-8"))
            ),
            Ok(())
        );
        assert_eq!(verdict(&check(301, None), &head(301, None)), Ok(()));
    }

    #[test]
    fn explains_mismatches() {
        assert_eq!(
            verdict(&check(200, None), &head(502, None)),
            Err("got 502, expected 200".to_owned())
        );
        assert_eq!(
            verdict(
                &check(200, Some("application/json")),
                &head(200, Some("text/html"))
            ),
            Err("content type text/html, expected application/json".to_owned())
        );
        assert_eq!(
            verdict(&check(200, Some("text/html")), &head(200, None)),
            Err("content type none, expected text/html".to_owned())
        );
    }
}
