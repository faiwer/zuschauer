//! Page handlers.

use axum::extract::State;
use maud::{DOCTYPE, Markup, html};

use crate::config::{App, Config};
use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp, now};
use crate::store::{Entity, store};

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

struct AppView {
    name: &'static str,
    checks: Vec<CheckRow>,
    containers: Vec<ContainerRow>,
}

pub async fn index(State(config): State<&'static Config>) -> Markup {
    let host = host_rows(config);
    let containers = container_rows();
    let apps: Vec<AppView> = config.apps.iter().map(app_view).collect();

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "Zuschauer" }
            }
            body {
                h1 { "Host" }
                table {
                    @for (label, sample) in host {
                        tr {
                            th { (label) }
                            @match sample {
                                Some(sample) => {
                                    td { strong { (format!("{:.1}%", sample.value)) } }
                                    td { (ago(sample.at)) " ago" }
                                }
                                None => td colspan="2" { "no data yet" },
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

fn host_rows(config: &Config) -> Vec<(String, Option<Sample>)> {
    let mut wanted = vec![
        ("CPU".to_owned(), EntityId::Host, MetricKind::Cpu),
        ("Memory".to_owned(), EntityId::Host, MetricKind::Memory),
    ];
    for mount in &config.host.storage {
        wanted.push((
            format!("Storage {}", mount.path.display()),
            EntityId::Mount(mount.path.clone()),
            MetricKind::Storage,
        ));
    }

    let store = store();
    wanted
        .into_iter()
        .map(|(label, entity, kind)| {
            let sample = store
                .series(&entity, kind)
                .and_then(|series| series.latest());
            (label, sample)
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
        .map(|name| container_row(name, store.entity(&EntityId::Container(name.clone()))))
        .collect();
    AppView {
        name: &app.name,
        checks,
        containers,
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
    let seconds = now().saturating_sub(at);
    match seconds {
        0..120 => format!("{seconds}s"),
        120..7200 => format!("{}m", seconds / 60),
        7200..172_800 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}
