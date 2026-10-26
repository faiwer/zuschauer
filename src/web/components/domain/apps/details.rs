use maud::{Markup, html};

use crate::config::Config;

use super::super::containers::render_container_item;
use super::app::{AppView, app_views};
use super::certificates::render_certificate_item;
use super::checks::render_check_item;
use super::commands::render_command_item;
use super::jobs::render_job_item;
use super::services::render_service_item;

/// The dialog's content for the app called `name`. `None`: no such app.
pub fn render_app_dialog(config: &'static Config, name: &str) -> Option<Markup> {
    let app = app_views(config).into_iter().find(|app| app.name == name)?;
    Some(render_details(&app))
}

fn render_details(app: &AppView) -> Markup {
    html! {
        @if !app.checks.is_empty() {
            section {
                h1 { "Health checks" }
                ol { @for row in &app.checks { (render_check_item(row)) } }
            }
        }
        @if !app.containers.is_empty() {
            section {
                h1 { "Containers" }
                ol { @for row in &app.containers { (render_container_item(row)) } }
            }
        }
        @if !app.certificates.is_empty() {
            section {
                h1 { "Certificates" }
                ol { @for row in &app.certificates { (render_certificate_item(row)) } }
            }
        }
        @if !app.services.is_empty() {
            section {
                h1 { "Services" }
                ol { @for row in &app.services { (render_service_item(row)) } }
            }
        }
        @if !app.commands.is_empty() {
            section {
                h1 { "Commands" }
                ol { @for row in &app.commands { (render_command_item(row)) } }
            }
        }
        @if !app.jobs.is_empty() {
            section {
                h1 { "Jobs" }
                ol { @for row in &app.jobs { (render_job_item(row)) } }
            }
        }
    }
}
