//! Page building blocks.
//!
//! - `ui`: generic pieces that know nothing about monitoring
//! - `domain`: the sections pages show, built from `ui`
//! - `styles`: every component's CSS file, inlined by `ui::frame`

mod domain;
mod styles;
mod ui;

pub use domain::{
    app_views, container_rows, header_view, host_view, render_alerts, render_apps,
    render_containers, render_header, render_host,
};
pub use ui::render_frame;
