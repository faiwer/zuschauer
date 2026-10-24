use maud::{Markup, html};

use crate::alerts::Firing;

use super::format::ago;

pub fn render_alerts(firing: &[Firing]) -> Markup {
    html! {
        h1 { "Alerts" }
        @if firing.is_empty() {
            p { "none firing" }
        } @else {
            table {
                @for alert in firing {
                    (render_alert_row(alert))
                }
            }
        }
    }
}

fn render_alert_row(alert: &Firing) -> Markup {
    html! {
        tr {
            th { (alert.alert.label) }
            td { strong { (alert.alert.severity) } }
            td { (alert.detail) }
            td { "for " (ago(alert.since)) }
        }
    }
}
