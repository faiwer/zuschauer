//! Scheduled jobs that report their runs over HTTP: `start`, then `ok` or
//! `fail`. Alerts when a run doesn't start on time, hangs or fails.
//!
//! - `state`: one job's runs and what they mean for its alert
//! - `registry`: every job's state, the reports, and a task per job that
//!   sleeps until its next deadline

mod registry;
mod state;

pub use registry::{ReportError, report, spawn, state};
pub use state::{Finished, JobState, Verdict};
