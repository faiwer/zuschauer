//! Page handlers, each putting `components` together.
//!
//! - `index`: the dashboard
//! - `favicon`: the logo, for the browser tab

mod favicon;
mod index;

pub use favicon::favicon;
pub use index::index;
