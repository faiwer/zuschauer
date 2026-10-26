//! Commands: programs zuschauer runs, one task per command. Exit code 0 is
//! fine, anything else a problem.
//!
//! - `run`: the task for one command, and what a run's end means
//! - `process`: runs the program, with its timeout
//! - `output`: keeps the lines of output a command asks for

mod output;
mod process;
mod run;

pub use run::run;
