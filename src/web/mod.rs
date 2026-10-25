//! HTTP server: routes, server-rendered pages and SVG sparklines.
//!
//! - `server`: router and listener
//! - `pages`: page handlers
//! - `components`: the pieces pages are built from
//! - `api`: endpoints for programs, like the one jobs report to
//! - `svg`: sparklines
//! - `timing`: middleware noting in HTML how long it took to render

mod api;
mod components;
mod pages;
mod server;
mod svg;
mod timing;

pub use server::serve;
