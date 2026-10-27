use crate::config::Config;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::super::components::{
    render_app_dialog, render_container_dialog, render_host_dialog, render_mount_dialog,
    render_resolved_dialog, render_sensor_dialog,
};

/// An entity's details, as HTML for `entity-dialog.js` to show as is.
pub async fn entity(
    State(config): State<&'static Config>,
    Path((kind, id)): Path<(String, String)>,
) -> Response {
    let markup = match kind.as_str() {
        "alerts" if id == "resolved" => Some(render_resolved_dialog()),
        "app" => render_app_dialog(config, &id),
        "container" => render_container_dialog(&id),
        "host" => render_host_dialog(&id),
        "mount" => render_mount_dialog(config, &id),
        "sensor" => render_sensor_dialog(config, &id),
        _ => None,
    };
    markup.map_or(
        StatusCode::NOT_FOUND.into_response(),
        IntoResponse::into_response,
    )
}
