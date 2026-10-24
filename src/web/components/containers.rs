use maud::{Markup, html};

use crate::model::{EntityId, MetricKind, Sample, Status};
use crate::store::{Entity, store};

use super::format::{mib, status_label};

pub struct ContainerRow {
    name: String,
    /// `None` when it hasn't been seen since start.
    status: Option<Status>,
    cpu: Option<Sample>,
    memory: Option<Sample>,
}

/// Every container seen since start, configured or not.
pub fn container_rows() -> Vec<ContainerRow> {
    store()
        .entities()
        .filter_map(|(id, entity)| match id {
            EntityId::Container(name) => Some(container_row(name, Some(entity))),
            _ => None,
        })
        .collect()
}

pub fn container_row(name: &str, entity: Option<&Entity>) -> ContainerRow {
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

pub fn render_containers(rows: &[ContainerRow]) -> Markup {
    html! {
        h1 { "Containers" }
        @if rows.is_empty() {
            p { "none running" }
        } @else {
            table {
                @for row in rows {
                    (render_container_row(row))
                }
            }
        }
    }
}

pub fn render_container_row(row: &ContainerRow) -> Markup {
    html! {
        tr {
            th { (row.name) }
            @match row.status {
                None => td colspan="3" { "not seen" },
                Some(Status::Gone { .. }) => {
                    td { (status_label(row.status)) }
                    td colspan="2" { "—" }
                }
                Some(_) => {
                    td { (status_label(row.status)) }
                    td { (row.cpu.map_or("—".to_owned(), |s| format!("CPU {:.1}%", s.value))) }
                    td { (row.memory.map_or("—".to_owned(), |s| format!("mem {}", mib(s.value)))) }
                }
            }
        }
    }
}
