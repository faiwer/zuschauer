//! Generic building blocks, free of monitoring concepts.
//!
//! - `frame`: the HTML document around a page
//! - `panel`: the titled box around each section
//! - `empty`: the toast in place of an empty list
//! - `gauge`: a ring filled to a percentage
//! - `hint`: hidden hover hints, shown by `tooltip.js`
//! - `icon`: the SVG icons from `web/icons`, as a sprite and references to it
//! - `level`: how good something looks, and its tint
//! - `logo`: the colored logo, served as the favicon and shown from there
//! - `format`: durations, sizes and statuses as text

mod empty;
mod format;
mod frame;
mod gauge;
mod hint;
mod icon;
mod level;
mod logo;
mod panel;

pub use empty::{Tone, render_empty};
pub use format::{ago, date_time, mib, span, status_label, until};
pub use frame::render_frame;
pub use gauge::render_gauge;
pub use hint::{hint_id, render_hint};
pub use icon::{Icon, icon, tinted_icon};
pub use level::Level;
pub use logo::{LOGO_SVG, render_logo};
pub use panel::render_panel;
