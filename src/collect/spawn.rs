use crate::config::Config;

use super::{docker, host};

/// Must be called inside the tokio runtime.
pub fn spawn(config: &'static Config) {
    host::spawn(&config.host);
    if let Some(docker) = &config.docker {
        tokio::spawn(docker::run(docker));
    }
}
