use maud::{Markup, html};

use crate::config::Config;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::{Entity, store};

use super::format::ago;

pub struct HostRow {
    label: String,
    sample: Option<Sample>,
    unit: &'static str,
    error: Option<String>,
}

/// CPU and memory, then the configured mounts and sensors.
pub fn host_rows(config: &Config) -> Vec<HostRow> {
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

pub fn render_host(rows: &[HostRow]) -> Markup {
    html! {
        h1 { "Host" }
        table {
            @for row in rows {
                (render_host_row(row))
            }
        }
    }
}

fn render_host_row(row: &HostRow) -> Markup {
    html! {
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
