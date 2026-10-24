//! Page building blocks. Each section gathers its rows from the store and
//! config (`*_rows`, `*_row`), then renders them (`render_*`).
//!
//! - `frame`: the HTML document around a page
//! - `styles`: every component's CSS file, inlined by `frame`
//! - `alerts`, `heartbeat`, `host`, `containers`, `apps`: the index sections
//! - `format`: durations, sizes and statuses as text

mod alerts;
mod apps;
mod containers;
mod format;
mod frame;
mod heartbeat;
mod host;
mod styles;

pub use alerts::render_alerts;
pub use apps::{app_views, render_apps};
pub use containers::{container_rows, render_containers};
pub use frame::render_frame;
pub use heartbeat::render_heartbeat;
pub use host::{host_rows, render_host};
