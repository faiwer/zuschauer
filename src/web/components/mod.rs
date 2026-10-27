//! Page building blocks.
//!
//! - `ui`: generic pieces that know nothing about monitoring
//! - `domain`: the sections pages show, built from `ui`
//! - `styles`: every component's CSS file, inlined by `ui::frame`

mod domain;
mod styles;
mod ui;

pub use domain::{
    app_views, container_rows, header_view, host_view, render_alerts, render_app_dialog,
    render_apps, render_container_dialog, render_containers, render_header, render_host,
    render_mount_dialog, render_sensor_dialog,
};
pub use ui::{LOGO_SVG, date, mib, render_frame, status_for, until};
