use crate::config::Config;

use super::{apps, docker, host};

/// Must be called inside the tokio runtime.
pub fn spawn(config: &'static Config) {
    host::spawn(&config.host);
    if let Some(docker) = &config.docker {
        tokio::spawn(docker::run(docker));
    }
    for app in &config.apps {
        for check in &app.health_checks {
            tokio::spawn(apps::run(app, check));
        }
    }
}
