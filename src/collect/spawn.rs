use crate::config::Config;

use super::apps::{certificate, health};
use super::{docker, host};

/// Must be called inside the tokio runtime.
pub fn spawn(config: &'static Config) {
    host::spawn(&config.host);
    if let Some(docker) = &config.docker {
        tokio::spawn(docker::run(docker));
    }
    for app in &config.apps {
        for check in &app.health_checks {
            tokio::spawn(health::run(app, check));
        }
        for target in &app.certificates {
            tokio::spawn(certificate::run(app, target, &config.certificates));
        }
    }
}
