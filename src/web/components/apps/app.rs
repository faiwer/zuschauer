use maud::{Markup, html};

use crate::config::{App, CertificatesConfig, Config};
use crate::model::EntityId;
use crate::store::store;

use super::super::containers::{ContainerRow, container_row, render_container_row};
use super::certificates::{CertificateRow, certificate_row, render_certificate_row};
use super::checks::{CheckRow, check_row, render_check_row};
use super::jobs::{JobRow, job_row, render_job_row};
use super::services::{ServiceRow, render_service_row, service_row};

pub struct AppView {
    name: &'static str,
    checks: Vec<CheckRow>,
    containers: Vec<ContainerRow>,
    certificates: Vec<CertificateRow>,
    services: Vec<ServiceRow>,
    jobs: Vec<JobRow>,
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
    html! {
        h1 { "Apps" }
        @for app in apps {
            (render_app(app))
        }
    }
}

fn render_app(app: &AppView) -> Markup {
    html! {
        h2 { (app.name) }
        table {
            @for row in &app.checks { (render_check_row(row)) }
            @for row in &app.containers { (render_container_row(row)) }
            @for row in &app.certificates { (render_certificate_row(row)) }
            @for row in &app.services { (render_service_row(row)) }
            @for row in &app.jobs { (render_job_row(row)) }
        }
    }
}
