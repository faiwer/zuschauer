use std::time::Duration;

use crate::config::raw::{RawCertificates, RawConfig, RawDocker};
use crate::config::resolved::{CertificatesConfig, Config, DockerConfig};
use crate::config::types::Dur;

use super::alerts::link_alerts;
use super::apps::apps;
use super::every::{DEFAULT_EVERY, every};
use super::host::host;
use super::notifiers::notifiers;
use super::services::services_config;

/// Certificates change rarely; this only has to catch a renewal that stopped.
const DEFAULT_CERTIFICATES_EVERY: Duration = Duration::from_secs(6 * 3600);
const DEFAULT_WARN_BEFORE: Duration = Duration::from_secs(14 * 86400);

pub fn resolve(raw: RawConfig) -> Result<Config, Vec<String>> {
    let mut errors = Vec::new();
    let host = host(raw.host.unwrap_or_default(), &mut errors);
    let docker = raw.docker.map(|raw_docker| docker(raw_docker, &mut errors));
    let certificates = certificates(raw.certificates.unwrap_or_default(), &mut errors);
    let services = services_config(raw.services.unwrap_or_default(), &mut errors);
    let apps = apps(
        raw.apps.unwrap_or_default(),
        docker.is_some(),
        &services,
        &mut errors,
    );
    let notifiers = notifiers(raw.notifiers.unwrap_or_default(), &mut errors);
    if raw.context.as_deref().is_some_and(str::is_empty) {
        errors.push("context: can't be empty, leave it out instead".to_owned());
    }

    let mut config = Config {
        listen: raw.listen,
        context: raw.context.unwrap_or_else(hostname),
        host,
        docker,
        certificates,
        services,
        apps,
        notifiers,
    };
    link_alerts(&mut config, &mut errors);

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(config)
}

fn docker(raw: RawDocker, errors: &mut Vec<String>) -> DockerConfig {
    if let Some(socket) = &raw.socket
        && !socket.exists()
    {
        errors.push(format!(
            "docker.socket: {} does not exist",
            socket.display()
        ));
    }
    DockerConfig {
        every: every("docker", raw.every, DEFAULT_EVERY, errors),
        socket: raw.socket,
    }
}

fn certificates(raw: RawCertificates, errors: &mut Vec<String>) -> CertificatesConfig {
    let warn_before = raw.warn_before.map_or(DEFAULT_WARN_BEFORE, Dur::get);
    if warn_before.is_zero() {
        errors.push("certificates.warnBefore: must be more than 0".to_owned());
    }
    CertificatesConfig {
        every: every(
            "certificates",
            raw.every,
            DEFAULT_CERTIFICATES_EVERY,
            errors,
        ),
        warn_before,
    }
}

fn hostname() -> String {
    let mut buf = [0u8; 256];
    // SAFETY: `buf` is writable for its whole length.
    if unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) } != 0 {
        return "zuschauer".to_owned();
    }
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn applies_defaults() {
        let config = resolve(parse(r#"{ "listen": "127.0.0.1:9100" }"#)).unwrap();
        assert_eq!(config.host.cpu.every, DEFAULT_EVERY);
        assert!(config.host.cpu.alert.is_none());
        assert!(config.host.storage.is_empty());
        assert!(!config.context.is_empty());
    }

    #[test]
    fn checks_certificates_settings() {
        let config = resolve(parse(r#"{ "listen": "127.0.0.1:9100" }"#)).unwrap();
        assert_eq!(config.certificates.every, DEFAULT_CERTIFICATES_EVERY);

        let errors = resolve(parse(
            r#"{ "listen": "127.0.0.1:9100", "certificates": { "warnBefore": "0s" } }"#,
        ))
        .unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:#?}");
    }

    #[test]
    fn rejects_unknown_fields() {
        let result: Result<RawConfig, _> =
            serde_json::from_str(r#"{ "listen": "127.0.0.1:9100", "hots": {} }"#);
        assert!(result.is_err());
    }
}
