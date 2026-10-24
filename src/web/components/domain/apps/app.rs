use maud::{Markup, html};

use crate::config::{App, CertificatesConfig, Config};
use crate::model::EntityId;
use crate::store::store;

use super::super::super::ui::{Icon, Level, icon, mib, render_panel, tinted_icon};
use super::super::containers::{ContainerRow, container_row, render_container_row};
use super::certificates::{CertificateRow, certificate_row, render_certificate_row};
use super::checks::{CheckRow, check_row, render_check_row};
use super::jobs::{JobRow, job_row, render_job_row};
use super::services::{ServiceRow, render_service_row, service_row};
use super::summary::{Verdicts, summarize};

pub struct AppView {
    pub(super) name: &'static str,
    pub(super) checks: Vec<CheckRow>,
    pub(super) containers: Vec<ContainerRow>,
    pub(super) certificates: Vec<CertificateRow>,
    pub(super) services: Vec<ServiceRow>,
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
        jobs: app.jobs.iter().map(|job| job_row(app, job)).collect(),
    }
}

pub fn render_apps(apps: &[AppView]) -> Markup {
    render_panel(
        Icon::App,
        "Apps",
        "apps",
        html! {
            table {
                thead {
                    tr {
                        th colspan="2" { "Name" }
                        th.app-narrow { "Memory" }
                        th.app-narrow { "CPU" }
                        th.app-narrow {}
                        th.app-narrow {}
                    }
                }
                tbody {
                    @for app in apps {
                        (render_app_row(app))
                    }
                }
            }
        },
    )
}

/// The worst of an app's checks, containers, services and jobs.
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
    html! {
        tr.table-row {
            td.app-narrow title=(health_title(&summary.health)) {
                (tinted_icon(Icon::App, level.class()))
            }
            td.highlight.(level.text_class()) { (app.name) }
            td.app-narrow {
                (summary.memory_bytes.map_or("—".to_owned(), mib))
            }
            td.app-narrow {
                (summary.cpu.map_or("—".to_owned(), |cpu| format!("{cpu:.1}%")))
            }
            td.app-narrow {
                @if let Some(certificates) = &summary.certificates {
                    span title=(certificates.notes.join("\n")) {
                        (tinted_icon(Icon::Cert, certificates.level.class()))
                    }
                }
            }
            td.app-narrow {
                button.icon-button type="button" title="Details" aria-label="Details" {
                    (icon(Icon::Eye))
                }
            }
        }
    }
}

fn health_title(health: &Verdicts) -> String {
    match health.level {
        Level::Unknown => "No data yet".to_owned(),
        Level::Ok => "All fine".to_owned(),
        Level::Warning | Level::Problem => health.notes.join(", "),
    }
}

#[expect(dead_code, reason = "the details popup will show it")]
fn render_app_details(app: &AppView) -> Markup {
    html! {
        div.app {
            h3.app-title { (app.name) }
            table {
                @for row in &app.checks { (render_check_row(row)) }
                @for row in &app.containers { (render_container_row(row)) }
                @for row in &app.certificates { (render_certificate_row(row)) }
                @for row in &app.services { (render_service_row(row)) }
                @for row in &app.jobs { (render_job_row(row)) }
            }
        }
    }
}
