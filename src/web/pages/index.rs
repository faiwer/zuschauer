use axum::extract::State;
use maud::{Markup, html};

use crate::alerts;
use crate::config::Config;
use crate::heartbeat;

use super::super::components::{
    app_views, container_rows, host_rows, render_alerts, render_apps, render_containers,
    render_frame, render_heartbeat, render_host,
};

pub async fn index(State(config): State<&'static Config>) -> Markup {
    let content = html! {
        @if config.alerts().next().is_some() {
            (render_alerts(&alerts::firing()))
        }
        @if let Some(heartbeat) = &config.heartbeat {
            (render_heartbeat(heartbeat, &heartbeat::status()))
        }
        (render_host(&host_rows(config)))
        @if config.docker.is_some() {
            (render_containers(&container_rows()))
        }
        @if !config.apps.is_empty() {
            (render_apps(&app_views(config)))
        }
    };
    render_frame("Zuschauer", content)
}
