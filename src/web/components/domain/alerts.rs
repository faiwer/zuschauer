use maud::{Markup, html};

use crate::alerts::Firing;
use crate::config::{Dur, Severity};

use super::super::ui::{Icon, Tone, ago, date_time, render_empty, render_panel, tinted_icon};

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
    html! {
        tr.table-row {
            td.alert-icon title=(alert.alert.severity) {
                (tinted_icon(Icon::Alert, severity_tint(alert.alert.severity)))
            }
            td {
                div.highlight { (alert.alert.label) }
                div { (alert.detail) }
            }
            td title=(age_details(alert)) { "~" (ago(alert.since)) }
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

fn age_details(alert: &Firing) -> String {
    format!(
        "Problem since {}, fires after {}",
        date_time(alert.since),
        Dur::from(alert.alert.after)
    )
}
