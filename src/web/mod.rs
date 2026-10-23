//! HTTP server: routes, server-rendered pages and SVG sparklines.
//!
//! - `server`: router and listener
//! - `pages`: page handlers
//! - `svg`: sparklines

mod pages;
mod server;
mod svg;

pub use server::serve;
