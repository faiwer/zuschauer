//! Alerts: collectors report what they observed, the engine decides when an
//! alert fires or resolves, and each of those changes becomes one
//! notification.
//!
//! - `engine`: the per-alert state machine
//! - `global`: the process-wide engine, `observe()` and `firing()`
//! - `message`: a change's notification text

mod engine;
mod global;
mod message;

pub use engine::Observation;
pub use global::{firing, observe};
