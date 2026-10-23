//! Page handlers.

use axum::extract::State;
use maud::{DOCTYPE, Markup, html};

use crate::config::Config;
use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp, now};
use crate::store::store;

struct ContainerRow {
    name: String,
    status: Option<Status>,
    cpu: Option<Sample>,
    memory: Option<Sample>,
}

pub async fn index(State(config): State<&'static Config>) -> Markup {
    let host = host_rows(config);
    let containers = container_rows();

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
                            @for row in containers {
                                tr {
                                    th { (row.name) }
                                    td { (status(row.status)) }
                                    @if matches!(row.status, Some(Status::Gone { .. })) {
                                        td colspan="2" { "—" }
                                    } @else {
                                        td { (row.cpu.map_or("—".to_owned(), |s| format!("CPU {:.1}%", s.value))) }
                                        td { (row.memory.map_or("—".to_owned(), |s| format!("mem {}", mib(s.value)))) }
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
            EntityId::Container(name) => Some(ContainerRow {
                name: name.clone(),
                status: entity.status(),
                cpu: entity.series(MetricKind::Cpu).and_then(|s| s.latest()),
                memory: entity
                    .series(MetricKind::MemoryBytes)
                    .and_then(|s| s.latest()),
            }),
            _ => None,
        })
        .collect()
}

fn status(status: Option<Status>) -> String {
    match status {
        Some(Status::Running) | None => "running".to_owned(),
        Some(Status::Paused) => "paused".to_owned(),
        Some(Status::Gone { since }) => format!("gone {}", ago(since)),
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
