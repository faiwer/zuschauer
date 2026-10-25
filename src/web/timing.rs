use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Ends HTML responses with how long the handler took, as a comment.
pub async fn timing(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let response = next.run(request).await;
    let html = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if !html {
        return response;
    }
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, usize::MAX).await else {
        return parts.into_response();
    };
    let mut bytes = bytes.to_vec();
    bytes.extend_from_slice(format!("<!-- generated in {ms:.1} ms -->").as_bytes());
    parts.headers.remove(CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(bytes))
}
