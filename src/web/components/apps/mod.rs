//! An app's section: one table with a row per health check, container,
//! certificate, service and job.
//!
//! - `app`: `AppView`, gathered per app, and the section around the rows
//! - `checks`, `certificates`, `services`, `jobs`: one kind of row each;
//!   container rows come from `containers`

mod app;
mod certificates;
mod checks;
mod jobs;
mod services;

pub use app::{app_views, render_apps};
