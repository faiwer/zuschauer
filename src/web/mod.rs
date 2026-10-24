//! HTTP server: routes, server-rendered pages and SVG sparklines.
//!
//! - `server`: router and listener
//! - `pages`: page handlers
//! - `components`: the pieces pages are built from
//! - `api`: endpoints for programs, like the one jobs report to
//! - `svg`: sparklines

mod api;
mod components;
mod pages;
mod server;
mod svg;

pub use server::serve;
