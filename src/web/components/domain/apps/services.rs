use maud::{Markup, html};

use crate::config::{App, AppService, Dur, RestartLimit};
use crate::model::{EntityId, MetricKind, Sample, Status};
use crate::store::{Entity, Store};

use super::super::super::ui::{Icon, Level, status_for, tinted_icon};

pub struct ServiceRow {
    pub(super) name: &'static str,
    pub(super) status: Option<Status>,
    /// Within the restart window, when the alert sets one.
    pub(super) restarts: Option<(Sample, RestartLimit)>,
    pub(super) error: Option<String>,
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

impl ServiceRow {
    fn level(&self) -> Level {
        match self.status {
            None => Level::Unknown,
            Some(Status::Down { .. }) => Level::Problem,
            Some(_) => Level::Ok,
        }
    }
}

/// The status icon, name and state, then the error while down, or the
/// restarts once past half the alert's limit.
pub fn render_service_item(row: &ServiceRow) -> Markup {
    let level = row.level();
    html! {
        li {
            (tinted_icon(Icon::Service, level.class(), None))
            " "
            span.highlight.(level.text_class()) { (row.name) }
            " — " (status_for(row.status))
            @if let Some(error) = &row.error {
                ", " (error)
            } @else if let Some((sample, limit)) = row.restarts
                && sample.value * 2.0 > limit.above as f32
            {
                ", " (format!("{:.0}", sample.value)) " of " (limit.above)
                " restarts in " (Dur::from(limit.within))
            }
        }
    }
}
