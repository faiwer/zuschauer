use maud::{Markup, html};

use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp};
use crate::store::{Entity, store};

use super::super::ui::{
    Icon, Level, Tone, ago, date_time, hint_id, mib, render_empty, render_graph, render_hint,
    render_panel, tinted_icon,
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
    let status_hint = hint_id();
    html! {
        tr.table-row.clickable data-entity=(format!("container:{}", row.name)) {
            td.container-icon {
                (tinted_icon(Icon::Container, level.class(), Some(&status_hint)))
                (render_hint(&status_hint, html! { (status_title(row.status)) }))
            }
            td.highlight.(level.text_class()) {
                span data-hint=[level.is_bad().then_some(&status_hint)]{ (row.name) }
            }
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

/// The dialog's content for the container called `name`, among those seen
/// since start. `None`: no such container.
pub fn render_container_dialog(name: &str) -> Option<Markup> {
    let row = container_rows().into_iter().find(|row| row.name == name)?;
    let (cpu, memory) = {
        let store = store();
        let entity = store.entity(&EntityId::Container(name.to_owned()));
        (
            series_points(entity, MetricKind::Cpu, 1.0),
            series_points(entity, MetricKind::MemoryBytes, 1.0 / MIB),
        )
    };
    Some(html! {
        section {
            h1 { "Container" }
            ol { (render_container_item(&row)) }
        }
        @if !cpu.is_empty() {
            section.container-graph {
                h1 { "CPU" }
                (render_graph(&cpu, "%"))
            }
        }
        @if !memory.is_empty() {
            section.container-graph {
                h1 { "Memory" }
                (render_graph(&memory, " MiB"))
            }
        }
    })
}

const MIB: f32 = 1024.0 * 1024.0;

/// The entity's series, oldest first, with values times `scale`.
fn series_points(entity: Option<&Entity>, metric: MetricKind, scale: f32) -> Vec<(Timestamp, f32)> {
    entity
        .and_then(|e| e.series(metric))
        .map(|series| series.iter().map(|s| (s.at, s.value * scale)).collect())
        .unwrap_or_default()
}

/// The status icon and name, then the usage while running, or the state.
pub fn render_container_item(row: &ContainerRow) -> Markup {
    let level = row.level();
    html! {
        li {
            (tinted_icon(Icon::Container, level.class(), None))
            " "
            span.highlight.(level.text_class()) { (row.name) }
            " — " (container_state(row))
        }
    }
}

fn container_state(row: &ContainerRow) -> String {
    match row.status {
        None => "not seen since start".to_owned(),
        Some(Status::Paused) => "paused".to_owned(),
        Some(Status::Gone { since } | Status::Down { since }) => {
            format!("gone at {}", date_time(since))
        }
        Some(Status::Running | Status::Up { .. }) => {
            let usage: Vec<String> = [
                row.cpu.map(|s| format!("{:.1}% CPU", s.value)),
                row.memory.map(|s| format!("{} RAM", mib(s.value))),
            ]
            .into_iter()
            .flatten()
            .collect();
            if usage.is_empty() {
                "running".to_owned()
            } else {
                usage.join(", ")
            }
        }
    }
}
