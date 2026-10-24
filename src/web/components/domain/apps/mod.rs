//! An app's section: one table with a row per health check, container,
//! certificate, service and job.
//!
//! - `app`: `AppView`, gathered per app, and the section around the rows
//! - `checks`, `certificates`, `services`, `jobs`: one kind of row each;
//!   container rows come from `containers`
//! - `summary`: an app's health, usage and certificates, for its table row

mod app;
mod certificates;
mod checks;
mod jobs;
mod services;
mod summary;

pub use app::{app_containers, app_level, app_views, render_apps};
