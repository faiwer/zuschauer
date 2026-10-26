use crate::config::Config;

use super::apps::{certificate, commands, health, services};
use super::{docker, host};

/// Which collectors to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    pub host: bool,
    pub containers: bool,
    pub checks: bool,
    pub certificates: bool,
    pub services: bool,
    pub commands: bool,
}

impl Parts {
    pub const ALL: Self = Self {
        host: true,
        containers: true,
        checks: true,
        certificates: true,
        services: true,
        commands: true,
    };
    pub const NONE: Self = Self {
        host: false,
        containers: false,
        checks: false,
        certificates: false,
        services: false,
        commands: false,
    };
}

pub fn spawn(config: &'static Config, parts: Parts) {
    if parts.host {
        host::spawn(&config.host);
    }
    if parts.containers
        && let Some(docker) = &config.docker
    {
        tokio::spawn(docker::run(docker, &config.apps));
    }
    for app in &config.apps {
        if parts.checks {
            for check in &app.health_checks {
                tokio::spawn(health::run(app, check));
            }
        }
        if parts.certificates {
            for watched in &app.certificates {
                tokio::spawn(certificate::run(app, watched, &config.certificates));
            }
        }
        if parts.commands {
            for command in &app.commands {
                tokio::spawn(commands::run(app, command));
            }
        }
    }
    if parts.services && config.apps.iter().any(|app| !app.services.is_empty()) {
        tokio::spawn(services::run(&config.services, &config.apps));
    }
}
