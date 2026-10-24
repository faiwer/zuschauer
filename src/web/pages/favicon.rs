use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::IntoResponse;

use super::super::components::LOGO_SVG;

pub async fn favicon() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "image/svg+xml"),
            (CACHE_CONTROL, "max-age=86400"),
        ],
        LOGO_SVG,
    )
}
