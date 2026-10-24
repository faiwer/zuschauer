use axum::extract::State;
use maud::{Markup, html};

use crate::alerts;
use crate::config::Config;

use super::super::components::{
    app_views, container_rows, header_view, host_view, render_alerts, render_apps,
    render_containers, render_frame, render_header, render_host,
};

pub async fn index(State(config): State<&'static Config>) -> Markup {
    let content = html! {
        (render_header(&header_view(config)))
        div.panel-row {
            (render_alerts(&alerts::firing(), config.alerts().next().is_some()))
            (render_host(&host_view(config)))
        }
        div.panel-row {
            (render_containers(&container_rows(), config.docker.is_some()))
            (render_apps(&app_views(config)))
        }
    };
    render_frame("Zuschauer", content)
}
