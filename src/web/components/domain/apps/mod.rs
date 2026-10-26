//! An app's section: one table with a row per health check, container,
//! certificate, service, command and job.
//!
//! - `app`: `AppView`, gathered per app, and the section around the rows
//! - `checks`, `certificates`, `services`, `commands`, `jobs`: one kind of
//!   row each; container rows come from `containers`
//! - `summary`: an app's health, usage and certificates, for its table row
//! - `details`: an app's groups of rows, for the entity dialog
mod app;
mod certificates;
mod checks;
mod commands;
mod details;
mod jobs;
mod services;
mod summary;

pub use app::{app_containers, app_level, app_views, render_apps};
pub use details::render_app_dialog;
