//! `POST /jobs/{app}/{job}/{event}`, where jobs report their runs.

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, header};

use crate::jobs::{self, ReportError};

pub async fn report(
    Path((app, job, event)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> (StatusCode, &'static str) {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim());
    match jobs::report(&app, &job, &event, token) {
        Ok(()) => (StatusCode::OK, "ok\n"),
        Err(ReportError::NotFound) => (
            StatusCode::NOT_FOUND,
            "no such job, or the event isn't start, ok or fail\n",
        ),
        Err(ReportError::Unauthorized) => (
            StatusCode::UNAUTHORIZED,
            "missing or wrong \"Authorization: Bearer <token>\"\n",
        ),
        Err(ReportError::NoRun) => (StatusCode::CONFLICT, "no run started, send start first\n"),
    }
}
