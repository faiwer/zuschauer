use maud::{Markup, html};

use crate::config::{App, CertificatesConfig, Config};
use crate::model::EntityId;
use crate::store::store;

use super::super::super::ui::{
    Icon, Level, Tone, hint_id, mib, render_empty, render_hint, render_panel, tinted_icon,
};
use super::super::containers::{ContainerRow, container_row};
use super::certificates::{CertificateRow, certificate_row};
use super::checks::{CheckRow, check_row};
use super::commands::{CommandRow, command_row};
use super::jobs::{JobRow, job_row};
use super::services::{ServiceRow, service_row};
use super::summary::{Verdicts, counted_containers, summarize};

pub struct AppView {
    pub(super) name: &'static str,
    pub(super) checks: Vec<CheckRow>,
    pub(super) containers: Vec<ContainerRow>,
    pub(super) certificates: Vec<CertificateRow>,
    pub(super) services: Vec<ServiceRow>,
    pub(super) commands: Vec<CommandRow>,
    pub(super) jobs: Vec<JobRow>,
}

pub fn app_views(config: &'static Config) -> Vec<AppView> {
    config
        .apps
        .iter()
        .map(|app| app_view(app, &config.certificates))
        .collect()
}

fn app_view(app: &'static App, certificates: &CertificatesConfig) -> AppView {
    let store = store();
    AppView {
        name: &app.name,
        checks: app
            .health_checks
            .iter()
            .map(|check| check_row(&store, app, check))
            .collect(),
        containers: app
            .containers
            .iter()
            .map(|c| container_row(&c.name, store.entity(&EntityId::Container(c.name.clone()))))
            .collect(),
        certificates: app
            .certificates
            .iter()
            .map(|watched| certificate_row(&store, app, watched, certificates))
            .collect(),
        services: app
            .services
            .iter()
            .map(|service| service_row(&store, app, service))
            .collect(),
        commands: app
            .commands
            .iter()
            .map(|command| command_row(&store, app, command))
            .collect(),
        jobs: app.jobs.iter().map(|job| job_row(app, job)).collect(),
    }
}

pub fn render_apps(apps: &[AppView]) -> Markup {
    render_panel(
        Icon::App,
        "Apps",
        "apps",
        html! {
            @if apps.is_empty() {
                (render_empty(Tone::Neutral(Icon::App), "No apps configured", "Set up apps in the configuration"))
            } @else {
                table {
                    thead {
                        tr {
                            th colspan="2" { "Name" }
                            th.app-narrow { "Memory" }
                            th.app-narrow { "CPU" }
                            th.app-narrow {}
                        }
                    }
                    tbody {
                        @for app in apps {
                            (render_app_row(app))
                        }
                    }
                }
            }
        },
    )
}

/// The worst of an app's checks, containers, services, commands and jobs.
pub fn app_level(app: &AppView) -> Level {
    summarize(app).health.level
}

/// The containers configured for an app.
pub fn app_containers(app: &AppView) -> &[ContainerRow] {
    &app.containers
}

fn render_app_row(app: &AppView) -> Markup {
    let summary = summarize(app);
    let level = summary.health.level;
    let health_hint = hint_id();
    let certificates_hint = hint_id();
    html! {
        tr.table-row.clickable data-entity=(format!("app:{}", app.name)) {
            td.app-narrow {
                (tinted_icon(Icon::App, level.class(), Some(&health_hint)))
                (render_hint(&health_hint, render_health_hint(&summary.health)))
            }
            td.highlight.(level.text_class()) {
                span data-hint=[level.is_bad().then_some(&health_hint)] { (app.name) }
            }
            td.app-narrow {
                (render_usage(app, summary.memory_bytes, |c| c.memory.map(|s| s.value), mib))
            }
            td.app-narrow {
                (render_usage(app, summary.cpu, |c| c.cpu.map(|s| s.value), percent))
            }
            td.app-narrow {
                @if let Some(certificates) = &summary.certificates {
                    (tinted_icon(Icon::Cert, certificates.level.class(), Some(&certificates_hint)))
                    (render_hint(&certificates_hint, html! {
                        ul { @for note in &certificates.notes { li { (note) } } }
                    }))
                }
            }
        }
    }
}

/// The app's total, with a per-container hint when several add up to it.
fn render_usage(
    app: &AppView,
    total: Option<f32>,
    value: impl Fn(&ContainerRow) -> Option<f32>,
    format: impl Fn(f32) -> String,
) -> Markup {
    let parts: Vec<(&str, f32)> = counted_containers(app)
        .filter_map(|c| Some((c.name.as_str(), value(c)?)))
        .collect();
    let hint = (parts.len() > 1).then(hint_id);
    html! {
        span data-hint=[&hint] { (total.map_or("—".to_owned(), &format)) }
        @if let Some(hint) = &hint {
            (render_hint(hint, html! {
                ul { @for (name, value) in &parts { li { (name) ": " (format(*value)) } } }
            }))
        }
    }
}

fn percent(value: f32) -> String {
    format!("{value:.1}%")
}

/// One line per note.
fn render_health_hint(health: &Verdicts) -> Markup {
    html! {
        @match health.level {
            Level::Unknown => "No data yet",
            Level::Ok => "All fine",
            Level::Warning | Level::Problem => {
                ul { @for note in &health.notes { li { (note) } } }
            }
        }
    }
}
