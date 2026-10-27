//! The sections pages show. Each gathers its rows from the store and config
//! (`*_rows`, `*_row`), then renders them (`render_*`) from `ui` pieces.
//!
//! - `header`: the site header, with the last update and the apps' health
//! - `heartbeat`: the header's heartbeat indicator
//! - `alerts`, `host`, `containers`, `apps`: the index sections
mod alerts;
mod apps;
mod containers;
mod header;
mod heartbeat;
mod host;

pub use alerts::render_alerts;
pub use apps::{app_views, render_app_dialog, render_apps};
pub use containers::{container_rows, render_container_dialog, render_containers};
pub use header::{header_view, render_header};
pub use host::{host_view, render_host, render_mount_dialog, render_sensor_dialog};
