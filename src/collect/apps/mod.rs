//! Apps: HTTP(S) health checks and TLS certificates, one task per check.
//! An app's containers come from the docker collector.
//!
//! - `health`: the task for one health check
//! - `verdict`: whether a response counts as healthy
//! - `certificate`: the task for one certificate
//! - `expiry`: reads the expiry date from a certificate
//! - `outcome`: stores an attempt's result, tracks up/down streaks

pub mod certificate;
mod expiry;
pub mod health;
mod outcome;
mod verdict;
