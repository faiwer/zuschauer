use maud::{Markup, html};

use crate::config::{App, AppService, Dur, RestartLimit};
use crate::model::{EntityId, MetricKind, Sample, Status};
use crate::store::{Entity, Store};

use super::super::format::status_label;

pub struct ServiceRow {
    name: &'static str,
    status: Option<Status>,
    /// Within the restart window, when the alert sets one.
    restarts: Option<(Sample, RestartLimit)>,
    error: Option<String>,
}

pub fn service_row(store: &Store, app: &App, service: &'static AppService) -> ServiceRow {
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
}

pub fn render_service_row(row: &ServiceRow) -> Markup {
    html! {
        tr {
            th { (row.name) }
            td { (status_label(row.status)) }
            @match row.restarts {
                Some((sample, limit)) => td {
                    (format!("{:.0}", sample.value)) " restarts in " (Dur::from(limit.within).to_string())
                },
                None => td { "—" },
            }
            td { (row.error.as_deref().unwrap_or_default()) }
        }
    }
}
