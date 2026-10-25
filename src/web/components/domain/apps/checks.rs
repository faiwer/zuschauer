use maud::{Markup, html};

use crate::config::{App, HealthCheck};
use crate::model::{EntityId, MetricKind, Sample, Status};
use crate::store::{Entity, Store};

use super::super::super::ui::{Icon, Level, status_for, tinted_icon};

pub struct CheckRow {
    pub(super) name: &'static str,
    pub(super) status: Option<Status>,
    pub(super) latency: Option<Sample>,
    pub(super) error: Option<String>,
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

impl CheckRow {
    fn level(&self) -> Level {
        match self.status {
            None => Level::Unknown,
            Some(Status::Down { .. }) => Level::Problem,
            Some(_) => Level::Ok,
        }
    }
}

/// The status icon, name and state, then the latency while up or the error
/// while down.
pub fn render_check_item(row: &CheckRow) -> Markup {
    let level = row.level();
    html! {
        li {
            (tinted_icon(Icon::Checkmark, level.class(), None))
            " "
            span.highlight.(level.text_class()) { (row.name) }
            " — " (status_for(row.status))
            @match (row.status, row.latency, &row.error) {
                (Some(Status::Up { .. }), Some(latency), _) => {
                    ", latency: " (format!("{:.0} ms", latency.value))
                }
                (Some(Status::Down { .. }), _, Some(error)) => { ": " (error) }
                _ => {}
            }
        }
    }
}
