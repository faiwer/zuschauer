use std::time::{Duration, Instant};

use tokio::time::sleep;

use crate::collect::{CERTIFICATE_TIMEOUT, Parts};
use crate::config::Config;
use crate::model::{EntityId, MetricKind, Status};
use crate::store::{Entity, store};

const POLL: Duration = Duration::from_millis(100);
/// Containers' CPU is the difference of two readings a second apart, and
/// nothing tells when the first one was taken.
const CONTAINERS_WINDOW: Duration = Duration::from_millis(1500);
/// Collectors that fail only log it, so a failure ends the wait only here.
const SHORTEST_LIMIT: Duration = Duration::from_secs(5);
/// Room for a request that ran into its timeout to report it.
const MARGIN: Duration = Duration::from_secs(1);

/// Until every selected reading is in, or the slowest of them would have
/// timed out.
pub async fn settle(config: &Config, parts: Parts) {
    let started = Instant::now();
    let limit = limit(config, parts);
    while started.elapsed() < limit && !ready(config, parts, started.elapsed()) {
        sleep(POLL).await;
    }
}

fn limit(config: &Config, parts: Parts) -> Duration {
    let apps = config.apps.iter();
    let checks = apps
        .clone()
        .filter(|_| parts.checks)
        .flat_map(|app| &app.health_checks)
        .map(|check| check.timeout);
    let certificates = apps
        .clone()
        .filter(|_| parts.certificates)
        .flat_map(|app| &app.certificates)
        .map(|_| CERTIFICATE_TIMEOUT);
    let commands = apps
        .filter(|_| parts.commands)
        .flat_map(|app| &app.commands)
        .map(|command| command.timeout);
    let slowest = checks
        .chain(certificates)
        .chain(commands)
        .max()
        .unwrap_or_default();
    SHORTEST_LIMIT.max(slowest + MARGIN)
}

fn ready(config: &Config, parts: Parts, elapsed: Duration) -> bool {
    let store = store();
    let entity = |id: EntityId| store.entity(&id);
    let has = |id: EntityId, metric: MetricKind| {
        entity(id).is_some_and(|e| e.series(metric).is_some_and(|s| s.latest().is_some()))
    };
    let answered = |id: EntityId| entity(id).is_some_and(|e| e.status().is_some());

    let host = !parts.host
        || has(EntityId::Host, MetricKind::Cpu)
            && has(EntityId::Host, MetricKind::Memory)
            && config
                .host
                .storage
                .iter()
                .all(|mount| has(EntityId::Mount(mount.path.clone()), MetricKind::Storage))
            && config.host.temperature.iter().all(|sensor| {
                let id = EntityId::Sensor(sensor.id.clone());
                entity(id.clone()).is_some_and(|e| e.error().is_some())
                    || has(id, MetricKind::Temperature)
            });
    let containers = !parts.containers
        || config.docker.is_none()
        || elapsed >= CONTAINERS_WINDOW
            && store.entities().all(|(id, e)| match id {
                EntityId::Container(_) => e.status() != Some(Status::Running) || has_cpu(e),
                _ => true,
            });
    let apps = config.apps.iter().all(|app| {
        let checks = !parts.checks
            || app.health_checks.iter().all(|check| {
                answered(EntityId::HealthCheck {
                    app: app.id.clone(),
                    check: check.name.clone(),
                })
            });
        let certificates = !parts.certificates
            || app.certificates.iter().all(|watched| {
                answered(EntityId::Certificate {
                    app: app.id.clone(),
                    host: watched.target.to_string(),
                })
            });
        let services = !parts.services
            || app.services.iter().all(|service| {
                answered(EntityId::Service {
                    app: app.id.clone(),
                    service: service.name.clone(),
                })
            });
        let commands = !parts.commands
            || app.commands.iter().all(|command| {
                answered(EntityId::Command {
                    app: app.id.clone(),
                    command: command.name.clone(),
                })
            });
        checks && certificates && services && commands
    });
    host && containers && apps
}

fn has_cpu(entity: &Entity) -> bool {
    entity
        .series(MetricKind::Cpu)
        .is_some_and(|s| s.latest().is_some())
}
