//! Page handlers, each putting `components` together.
//!
//! - `index`: the dashboard
//! - `favicon`: the logo, for the browser tab
//! - `entity`: an app's, container's or host reading's details, or the
//!   resolved alerts, for the dialog

mod entity;
mod favicon;
mod index;

pub use entity::entity;
pub use favicon::favicon;
pub use index::index;
