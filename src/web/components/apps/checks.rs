use maud::{Markup, html};

use crate::config::{App, HealthCheck};
use crate::model::{EntityId, MetricKind, Sample, Status};
use crate::store::{Entity, Store};

use super::super::format::status_label;

pub struct CheckRow {
    name: &'static str,
    status: Option<Status>,
    latency: Option<Sample>,
    error: Option<String>,
}

pub fn check_row(store: &Store, app: &App, check: &'static HealthCheck) -> CheckRow {
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
}

pub fn render_check_row(row: &CheckRow) -> Markup {
    html! {
        tr {
            th { (row.name) }
            td { (status_label(row.status)) }
            @match (row.status, row.latency) {
                (Some(Status::Up { .. }), Some(s)) => td { "responded in " (format!("{:.0}", s.value)) " ms" },
                _ => td { "—" },
            }
            td { (row.error.as_deref().unwrap_or_default()) }
        }
    }
}
