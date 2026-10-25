use maud::{Markup, html};

use crate::config::{Alert, Config};
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::{Entity, Store, store};

use super::super::ui::{
    Icon, Level, hint_id, icon, render_gauge, render_hint, render_panel, tinted_icon,
};

pub struct HostView {
    cpu: HostRow,
    memory: HostRow,
    /// The configured mounts, then the sensors.
    readings: Vec<(Icon, HostRow)>,
}

pub struct HostRow {
    pub(super) label: String,
    pub(super) sample: Option<Sample>,
    pub(super) unit: &'static str,
    pub(super) error: Option<String>,
    /// The alert's threshold, in the sample's unit.
    pub(super) above: Option<f32>,
}

pub fn host_view(config: &Config) -> HostView {
    let store = store();
    let storage = config.host.storage.iter().map(|mount| {
        let label = mount.path.display().to_string();
        let id = EntityId::Mount(mount.path.clone());
        let row = host_row(&store, label, &id, MetricKind::Storage, "%");
        (Icon::Storage, row.above(&mount.metric.alert))
    });
    let temperature = config.host.temperature.iter().map(|sensor| {
        let id = EntityId::Sensor(sensor.id.clone());
        let row = host_row(
            &store,
            sensor.name.clone(),
            &id,
            MetricKind::Temperature,
            " °C",
        );
        (Icon::Temperature, row.above(&sensor.alert))
    });
    let host = |label: &str, kind| host_row(&store, label.to_owned(), &EntityId::Host, kind, "%");
    HostView {
        cpu: host("CPU", MetricKind::Cpu),
        memory: host("Memory", MetricKind::Memory),
        readings: storage.chain(temperature).collect(),
    }
}

fn host_row(
    store: &Store,
    label: String,
    id: &EntityId,
    kind: MetricKind,
    unit: &'static str,
) -> HostRow {
    let entity = store.entity(id);
    HostRow {
        label,
        sample: entity.and_then(|e| e.series(kind)).and_then(|s| s.latest()),
        unit,
        error: entity.and_then(Entity::error).map(str::to_owned),
        above: None,
    }
}

impl HostRow {
    fn above(self, alert: &Option<Alert>) -> Self {
        let above = alert
            .as_ref()
            .and_then(|a| a.threshold.as_ref())
            .map(|t| t.above);
        Self { above, ..self }
    }

    /// A problem at the threshold, a warning from 75% of it.
    fn level(&self) -> Level {
        match (self.sample, self.above) {
            (Some(sample), Some(above)) if sample.value >= above => Level::Problem,
            (Some(sample), Some(above)) if sample.value >= above * 0.75 => Level::Warning,
            _ => Level::Unknown,
        }
    }

    fn value(&self) -> String {
        match self.sample {
            Some(sample) => format!("{:.1}{}", sample.value, self.unit),
            None => "—".to_owned(),
        }
    }
}

pub fn render_host(view: &HostView) -> Markup {
    render_panel(
        Icon::Host,
        "Host",
        "host",
        html! {
            div.host-gauges {
                (render_host_gauge(&view.cpu))
                (render_host_gauge(&view.memory))
            }
            @if !view.readings.is_empty() {
                table.host-readings {
                    @for (reading_icon, row) in &view.readings {
                        (render_reading_row(*reading_icon, row))
                    }
                }
            }
        },
    )
}

fn render_host_gauge(row: &HostRow) -> Markup {
    render_gauge(
        &row.label,
        row.sample.map(|s| s.value),
        &row.value(),
        row.error.as_deref(),
    )
}

fn render_reading_row(reading_icon: Icon, row: &HostRow) -> Markup {
    let hint = row.error.as_ref().map(|error| (hint_id(), error));
    let hint_ref = hint.as_ref().map(|(id, _)| id);
    html! {
        tr.table-row.host-reading-error[row.error.is_some()] {
            td.host-reading-icon {
                @if row.error.is_some() {
                    (tinted_icon(reading_icon, "tint-problem", hint_ref.map(String::as_str)))
                } @else {
                    (icon(reading_icon))
                }
                @if let Some((id, error)) = &hint {
                    (render_hint(id, html! { (error) }))
                }
            }
            td.highlight.host-reading-label data-hint=[hint_ref] { (row.label) }
            td.host-reading-value.(row.level().text_class()) { (row.value()) }
        }
    }
}
