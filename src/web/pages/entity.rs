use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use maud::html;

/// An entity's details, as HTML for `entity-dialog.js` to show as is.
pub async fn entity(Path((kind, id)): Path<(String, String)>) -> Response {
    match kind.as_str() {
        "app" | "container" => html! { b { (id) } }.into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
