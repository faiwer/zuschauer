//! Collectors. Each one runs as its own task on its own `every` timer and
//! writes samples into the store.

mod host;

use crate::config::Config;

/// Must be called inside the tokio runtime.
pub fn spawn(config: &'static Config) {
    host::spawn(&config.host);
}
