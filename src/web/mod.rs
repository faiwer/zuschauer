//! HTTP server: routes, server-rendered pages and SVG sparklines.
//!
//! - `server`: router and listener
//! - `pages`: page handlers
//! - `jobs`: the endpoint jobs report their runs to
//! - `svg`: sparklines

mod jobs;
mod pages;
mod server;
mod svg;

pub use server::serve;
