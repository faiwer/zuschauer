//! `--state`: runs the collectors, prints what they found, and exits, or
//! with `--every` prints again on that interval until killed. Jobs and
//! alerts are left out: they only mean something with history.
//!
//! - `parts`: the `--only` list
//! - `wait`: waits until every selected reading is in
//! - `print`: the readings as text
//! - `show`: ties them together

mod parts;
mod print;
mod show;
mod wait;

pub use parts::{NAMES, parse_parts};
pub use show::show;
