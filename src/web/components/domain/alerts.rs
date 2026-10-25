use maud::{Markup, html};

use crate::alerts::Firing;
use crate::config::{Dur, Severity};

use super::super::ui::{
    Icon, Tone, ago, date_time, hint_id, render_empty, render_hint, render_panel, tinted_icon,
};

/// `configured`: whether the config has any alert at all.
pub fn render_alerts(firing: &[Firing], configured: bool) -> Markup {
    render_panel(
        Icon::Alert,
        "Alerts",
        "alerts",
        html! {
            @if !configured {
                (render_empty(Tone::Warning, "No alerts configured", "Set up new alerts in the configuration"))
            } @else if firing.is_empty() {
                (render_empty(Tone::Positive, "All clear", "No alerts firing"))
            } @else {
                table {
                    @for alert in firing {
                        (render_alert_row(alert))
                    }
                }
            }
        },
    )
}

fn render_alert_row(alert: &Firing) -> Markup {
    let severity_hint = hint_id();
    let age_hint = hint_id();
    html! {
        tr.table-row {
            td.alert-icon {
                (tinted_icon(Icon::Alert, severity_tint(alert.alert.severity), Some(&severity_hint)))
                (render_hint(&severity_hint, html! { (alert.alert.severity) }))
            }
            td {
                div.highlight { (alert.alert.label) }
                div { (alert.detail) }
            }
            td.alert-age data-hint=(age_hint) {
                "~" (ago(alert.since))
                (render_hint(&age_hint, render_age_hint(alert)))
            }
        }
    }
}

fn severity_tint(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "tint-problem",
        Severity::Warning => "tint-warning",
        Severity::Info => "tint-info",
    }
}

fn render_age_hint(alert: &Firing) -> Markup {
    html! {
        ul {
            li { "Problem since " (date_time(alert.since)) }
            li { "Fires after " (Dur::from(alert.alert.after)) }
        }
    }
}
