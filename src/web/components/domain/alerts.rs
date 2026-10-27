use maud::{Markup, html};

use crate::alerts::{Firing, resolved};
use crate::config::{Dur, Severity};

use super::super::ui::{
    Icon, Tone, ago, date_time, hint_id, icon, render_empty, render_hint, render_panel_with, span,
    tinted_icon,
};

/// `configured`: whether the config has any alert at all.
pub fn render_alerts(firing: &[Firing], configured: bool) -> Markup {
    let history_hint = hint_id();
    let history = html! {
        button.icon-button type="button" aria-label="Resolved alerts"
            data-entity="alerts:resolved" data-hint=(history_hint) {
            (icon(Icon::Clock))
            (render_hint(&history_hint, html! { "Resolved alerts" }))
        }
    };
    render_panel_with(
        Icon::Alert,
        "Alerts",
        configured.then_some(history),
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
                (tinted_icon(Icon::Alert, severity_tint(alert.severity), Some(&severity_hint)))
                (render_hint(&severity_hint, html! { (alert.severity) }))
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

/// The dialog's content: the latest resolved alerts since start.
pub fn render_resolved_dialog() -> Markup {
    let list = resolved();
    html! {
        section {
            h1 { "Resolved alerts" }
            @if list.is_empty() {
                (render_empty(Tone::Neutral(Icon::Clock), "No resolved alerts", "None since start"))
            } @else {
                table.alerts-resolved {
                    thead {
                        tr {
                            th colspan="2" { "Alert" }
                            th { "Started" }
                            th { "Resolved" }
                            th { "Lasted" }
                        }
                    }
                    tbody {
                        @for alert in &list {
                            tr.table-row {
                                td.alert-icon {
                                    (tinted_icon(Icon::Alert, severity_tint(alert.severity), None))
                                }
                                td {
                                    div.highlight { (alert.alert.label) }
                                    div { (alert.detail) }
                                }
                                td.alert-time { (date_time(alert.since)) }
                                td.alert-time { (date_time(alert.ended)) }
                                td.alert-time { (span(alert.ended.saturating_sub(alert.since))) }
                            }
                        }
                    }
                }
            }
        }
    }
}
