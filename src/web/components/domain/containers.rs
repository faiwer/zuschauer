use maud::{Markup, html};

use crate::model::{EntityId, MetricKind, Sample, Status};
use crate::store::{Entity, store};

use super::super::ui::{
    Icon, Level, Tone, ago, mib, render_empty, render_panel, status_label, tinted_icon,
};

pub struct ContainerRow {
    pub(super) name: String,
    /// `None` when it hasn't been seen since start.
    pub(super) status: Option<Status>,
    pub(super) cpu: Option<Sample>,
    pub(super) memory: Option<Sample>,
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

/// `configured`: whether the config watches Docker at all.
pub fn render_containers(rows: &[ContainerRow], configured: bool) -> Markup {
    let tone = Tone::Neutral(Icon::Container);
    render_panel(
        Icon::Container,
        "Containers",
        "containers",
        html! {
            @if !configured {
                (render_empty(tone, "Docker not configured", "No containers are watched"))
            } @else if rows.is_empty() {
                (render_empty(tone, "No containers", "None seen since start"))
            } @else {
                table {
                    thead {
                        tr {
                            th colspan="2" { "Name" }
                            th.container-usage { "CPU" }
                            th.container-usage { "Memory" }
                        }
                    }
                    tbody {
                        @for row in rows {
                            (render_container_panel_row(row))
                        }
                    }
                }
            }
        },
    )
}

/// The containers panel's row: the status as the icon's color, then the
/// usage under the table's headers.
fn render_container_panel_row(row: &ContainerRow) -> Markup {
    let usage = !matches!(row.status, None | Some(Status::Gone { .. }));
    let level = row.level();
    html! {
        tr.table-row {
            td.container-icon title=(status_title(row.status)) {
                (tinted_icon(Icon::Container, level.class()))
            }
            td.highlight.(level.text_class()) { (row.name) }
            @if usage {
                td.container-usage { (row.cpu.map_or("—".to_owned(), |s| format!("{:.1}%", s.value))) }
                td.container-usage { (row.memory.map_or("—".to_owned(), |s| mib(s.value))) }
            } @else {
                td.container-usage { "—" }
                td.container-usage { "—" }
            }
        }
    }
}

impl ContainerRow {
    pub fn level(&self) -> Level {
        match self.status {
            None => Level::Unknown,
            Some(Status::Running | Status::Up { .. }) => Level::Ok,
            Some(Status::Paused) => Level::Warning,
            Some(Status::Gone { .. } | Status::Down { .. }) => Level::Problem,
        }
    }
}

fn status_title(status: Option<Status>) -> String {
    match status {
        None => "Not seen since start".to_owned(),
        Some(Status::Running | Status::Up { .. }) => "Running".to_owned(),
        Some(Status::Paused) => "Paused".to_owned(),
        Some(Status::Gone { since } | Status::Down { since }) => format!("Gone for {}", ago(since)),
    }
}

pub fn render_container_row(row: &ContainerRow) -> Markup {
    html! {
        tr.table-row {
            td.highlight { (row.name) }
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
