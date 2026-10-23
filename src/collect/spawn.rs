use crate::config::Config;

use super::apps::{certificate, health};
use super::{docker, host};

pub fn spawn(config: &'static Config) {
    host::spawn(&config.host);
    if let Some(docker) = &config.docker {
        tokio::spawn(docker::run(docker, &config.apps));
    }
    for app in &config.apps {
        for check in &app.health_checks {
            tokio::spawn(health::run(app, check));
        }
        for watched in &app.certificates {
            tokio::spawn(certificate::run(app, watched, &config.certificates));
        }
    }
}
