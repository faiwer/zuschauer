//! HTTP server: routes and server-rendered pages.
//!
//! - `server`: router and listener
//! - `pages`: page handlers
//! - `components`: the pieces pages are built from
//! - `api`: endpoints for programs, like the one jobs report to
//! - `timing`: middleware noting in HTML how long it took to render

mod api;
mod components;
mod pages;
mod server;
mod timing;

pub use components::{date, mib, status_for, until};
pub use server::serve;
