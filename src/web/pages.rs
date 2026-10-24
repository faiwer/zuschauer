//! Page handlers.

use axum::extract::State;
use maud::{DOCTYPE, Markup, html};

use crate::alerts;
use crate::config::{App, Config, Dur, RestartLimit};
use crate::cron::format_local;
use crate::jobs::{self, JobState, Verdict};
use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp, now};
use crate::store::{Entity, store};

struct HostRow {
    label: String,
    sample: Option<Sample>,
    unit: &'static str,
    error: Option<String>,
}

struct ContainerRow {
    name: String,
    /// `None` when it hasn't been seen since start.
    status: Option<Status>,
    cpu: Option<Sample>,
    memory: Option<Sample>,
}

struct CheckRow {
    name: &'static str,
    status: Option<Status>,
    latency: Option<Sample>,
    error: Option<String>,
}

struct CertificateRow {
    host: String,
    status: Option<Status>,
    expires_at: Option<Timestamp>,
    error: Option<String>,
}

struct ServiceRow {
    name: &'static str,
    status: Option<Status>,
    /// Within the restart window, when the alert sets one.
    restarts: Option<(Sample, RestartLimit)>,
    error: Option<String>,
}

struct JobRow {
    name: &'static str,
    state: Option<(JobState, Verdict)>,
}

struct AppView {
    name: &'static str,
    checks: Vec<CheckRow>,
    containers: Vec<ContainerRow>,
    certificates: Vec<CertificateRow>,
    services: Vec<ServiceRow>,
    jobs: Vec<JobRow>,
}

pub async fn index(State(config): State<&'static Config>) -> Markup {
    let firing = alerts::firing();
    let host = host_rows(config);
    let containers = container_rows();
    let apps: Vec<AppView> = config.apps.iter().map(app_view).collect();
    let warn_before =
        Timestamp::try_from(config.certificates.warn_before.as_secs()).unwrap_or(Timestamp::MAX);
    let warn_after = now().saturating_add(warn_before);

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "Zuschauer" }
            }
            body {
                @if config.alerts().next().is_some() {
                    h1 { "Alerts" }
                    @if firing.is_empty() {
                        p { "none firing" }
                    } @else {
                        table {
                            @for alert in &firing {
                                tr {
                                    th { (alert.alert.label) }
                                    td { strong { (alert.alert.severity) } }
                                    td { (alert.detail) }
                                    td { "for " (ago(alert.since)) }
                                }
                            }
                        }
                    }
                }
                h1 { "Host" }
                table {
                    @for row in &host {
                        tr {
                            th { (row.label) }
                            @match (row.sample, &row.error) {
                                (_, Some(error)) => td colspan="2" { (error) },
                                (Some(sample), None) => {
                                    td { strong { (format!("{:.1}{}", sample.value, row.unit)) } }
                                    td { (ago(sample.at)) " ago" }
                                }
                                (None, None) => td colspan="2" { "no data yet" },
                            }
                        }
                    }
                }
                @if config.docker.is_some() {
                    h1 { "Containers" }
                    @if containers.is_empty() {
                        p { "none running" }
                    } @else {
                        table {
                            @for row in &containers {
                                tr { (container_cells(row)) }
                            }
                        }
                    }
                }
                @if !apps.is_empty() {
                    h1 { "Apps" }
                    @for app in &apps {
                        h2 { (app.name) }
                        table {
                            @for row in &app.checks {
                                tr {
                                    th { (row.name) }
                                    td { (status(row.status)) }
                                    @match (row.status, row.latency) {
                                        (Some(Status::Up { .. }), Some(s)) => td { "responded in " (format!("{:.0}", s.value)) " ms" },
                                        _ => td { "—" },
                                    }
                                    td { (row.error.as_deref().unwrap_or_default()) }
                                }
                            }
                            @for row in &app.containers {
                                tr { (container_cells(row)) }
                            }
                            @for row in &app.certificates {
                                tr {
                                    th { (row.host) }
                                    @match (row.status, row.expires_at) {
                                        (Some(Status::Up { .. }), Some(at)) if at < warn_after => {
                                            td { strong { "expiring" } }
                                            td { "in " (until(at)) }
                                        }
                                        (Some(Status::Up { .. }), Some(at)) => {
                                            td { "valid" }
                                            td { "expires in " (until(at)) }
                                        }
                                        (Some(Status::Down { since }), _) => {
                                            td { "failing " (ago(since)) }
                                            td { "—" }
                                        }
                                        _ => {
                                            td { "pending" }
                                            td { "—" }
                                        }
                                    }
                                    td { (row.error.as_deref().unwrap_or_default()) }
                                }
                            }
                            @for row in &app.services {
                                tr {
                                    th { (row.name) }
                                    td { (status(row.status)) }
                                    @match row.restarts {
                                        Some((sample, limit)) => td {
                                            (format!("{:.0}", sample.value)) " restarts in " (Dur::from(limit.within).to_string())
                                        },
                                        None => td { "—" },
                                    }
                                    td { (row.error.as_deref().unwrap_or_default()) }
                                }
                            }
                            @for row in &app.jobs {
                                tr {
                                    th { (row.name) }
                                    @if let Some((state, verdict)) = &row.state {
                                        (job_cells(state, *verdict))
                                    } @else {
                                        td colspan="3" { "—" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn container_cells(row: &ContainerRow) -> Markup {
    html! {
        th { (row.name) }
        @match row.status {
            None => td colspan="3" { "not seen" },
            Some(Status::Gone { .. }) => {
                td { (status(row.status)) }
                td colspan="2" { "—" }
            }
            Some(_) => {
                td { (status(row.status)) }
                td { (row.cpu.map_or("—".to_owned(), |s| format!("CPU {:.1}%", s.value))) }
                td { (row.memory.map_or("—".to_owned(), |s| format!("mem {}", mib(s.value)))) }
            }
        }
    }
}

fn job_cells(state: &JobState, verdict: Verdict) -> Markup {
    html! {
        td {
            @match verdict {
                Verdict::Hung { since } => { strong { "hung" } ", running " (ago(since)) }
                Verdict::Failed { retrying: Some(since), .. } => {
                    strong { "failed" } ", retry running " (ago(since))
                }
                Verdict::Failed { retrying: None, .. } => strong { "failed" },
                Verdict::Late { .. } => strong { "late" },
                Verdict::Running { since } => { "running " (ago(since)) }
                Verdict::Waiting if state.last.is_some() => "ok",
                Verdict::Waiting => "waiting",
            }
        }
        td {
            @match state.last {
                Some(last) => {
                    (if last.ok { "succeeded " } else { "failed " }) (ago(last.finished)) " ago, took "
                    (span(last.finished.saturating_sub(last.started)))
                }
                None => "no runs yet",
            }
        }
        td {
            @match state.next_due {
                Some(due) => { "due " (format_local(due)) }
                None => "never due",
            }
        }
    }
}

fn host_rows(config: &Config) -> Vec<HostRow> {
    let mut wanted = vec![
        ("CPU".to_owned(), EntityId::Host, MetricKind::Cpu, "%"),
        ("Memory".to_owned(), EntityId::Host, MetricKind::Memory, "%"),
    ];
    for mount in &config.host.storage {
        wanted.push((
            format!("Storage {}", mount.path.display()),
            EntityId::Mount(mount.path.clone()),
            MetricKind::Storage,
            "%",
        ));
    }
    for sensor in &config.host.temperature {
        wanted.push((
            format!("Temperature {}", sensor.name),
            EntityId::Sensor(sensor.id.clone()),
            MetricKind::Temperature,
            " °C",
        ));
    }

    let store = store();
    wanted
        .into_iter()
        .map(|(label, id, kind, unit)| {
            let entity = store.entity(&id);
            HostRow {
                label,
                sample: entity.and_then(|e| e.series(kind)).and_then(|s| s.latest()),
                unit,
                error: entity.and_then(Entity::error).map(str::to_owned),
            }
        })
        .collect()
}

fn container_rows() -> Vec<ContainerRow> {
    store()
        .entities()
        .filter_map(|(id, entity)| match id {
            EntityId::Container(name) => Some(container_row(name, Some(entity))),
            _ => None,
        })
        .collect()
}

fn container_row(name: &str, entity: Option<&Entity>) -> ContainerRow {
    ContainerRow {
        name: name.to_owned(),
        status: entity.and_then(Entity::status),
        cpu: entity
            .and_then(|e| e.series(MetricKind::Cpu))
            .and_then(|s| s.latest()),
        memory: entity
            .and_then(|e| e.series(MetricKind::MemoryBytes))
            .and_then(|s| s.latest()),
    }
}

fn app_view(app: &'static App) -> AppView {
    let store = store();
    let checks = app
        .health_checks
        .iter()
        .map(|check| {
            let entity = store.entity(&EntityId::HealthCheck {
                app: app.id.clone(),
                check: check.name.clone(),
            });
            CheckRow {
                name: &check.name,
                status: entity.and_then(Entity::status),
                latency: entity
                    .and_then(|e| e.series(MetricKind::Latency))
                    .and_then(|s| s.latest()),
                error: entity.and_then(Entity::error).map(str::to_owned),
            }
        })
        .collect();
    let containers = app
        .containers
        .iter()
        .map(|c| container_row(&c.name, store.entity(&EntityId::Container(c.name.clone()))))
        .collect();
    let certificates = app
        .certificates
        .iter()
        .map(|watched| {
            let host = watched.target.to_string();
            let entity = store.entity(&EntityId::Certificate {
                app: app.id.clone(),
                host: host.clone(),
            });
            CertificateRow {
                host,
                status: entity.and_then(Entity::status),
                expires_at: entity.and_then(Entity::expires_at),
                error: entity.and_then(Entity::error).map(str::to_owned),
            }
        })
        .collect();
    let services = app
        .services
        .iter()
        .map(|service| {
            let entity = store.entity(&EntityId::Service {
                app: app.id.clone(),
                service: service.name.clone(),
            });
            let restarts = entity
                .and_then(|e| e.series(MetricKind::Restarts))
                .and_then(|s| s.latest());
            ServiceRow {
                name: &service.name,
                status: entity.and_then(Entity::status),
                restarts: restarts.zip(service.alert.as_ref().and_then(|a| a.restarts)),
                error: entity.and_then(Entity::error).map(str::to_owned),
            }
        })
        .collect();
    let jobs = app
        .jobs
        .iter()
        .map(|job| JobRow {
            name: &job.name,
            state: jobs::state(&app.id, &job.name).map(|mut state| {
                let verdict = state.judge(job, now());
                (state, verdict)
            }),
        })
        .collect();
    AppView {
        name: &app.name,
        checks,
        containers,
        certificates,
        services,
        jobs,
    }
}

fn status(status: Option<Status>) -> String {
    match status {
        None => "pending".to_owned(),
        Some(Status::Running) => "running".to_owned(),
        Some(Status::Paused) => "paused".to_owned(),
        Some(Status::Gone { since }) => format!("gone {}", ago(since)),
        Some(Status::Up { since }) => format!("up {}", ago(since)),
        Some(Status::Down { since }) => format!("down {}", ago(since)),
    }
}

fn mib(bytes: f32) -> String {
    format!("{:.0} MiB", bytes / (1024.0 * 1024.0))
}

/// Compact age: `42s`, `12m`, `5h`, `3d`.
fn ago(at: Timestamp) -> String {
    span(now().saturating_sub(at))
}

/// Like `ago`, for the future.
fn until(at: Timestamp) -> String {
    span(at.saturating_sub(now()))
}

fn span(seconds: Timestamp) -> String {
    match seconds {
        0..120 => format!("{seconds}s"),
        120..7200 => format!("{}m", seconds / 60),
        7200..172_800 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}
