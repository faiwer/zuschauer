use maud::{Markup, html};

use crate::config::Heartbeat;
use crate::heartbeat::Status;

use super::super::ui::{Icon, ago, hint_id, icon, render_hint};

/// The header's heartbeat indicator.
pub struct HeartbeatView {
    /// A `.tint-*` class.
    class: &'static str,
    hint: Markup,
}

pub fn heartbeat_view(heartbeat: &Heartbeat, status: &Status) -> HeartbeatView {
    let (class, state, error) = match (&status.failing, status.sent) {
        (Some((since, error)), sent) => {
            let sent = sent
                .map(|at| format!(", last sent {} ago", ago(at)))
                .unwrap_or_default();
            (
                "tint-problem",
                format!("Failing for {}{sent}", ago(*since)),
                Some(error),
            )
        }
        (None, Some(at)) => ("tint-ok", format!("Sent {} ago", ago(at)), None),
        (None, None) => ("tint-unknown", "Pending".to_owned(), None),
    };
    HeartbeatView {
        class,
        hint: html! {
            ul {
                li { "Heartbeat: " (heartbeat) }
                li { (state) }
                @if let Some(error) = error { li { (error) } }
            }
        },
    }
}

pub fn render_heartbeat(view: &HeartbeatView) -> Markup {
    let hint = hint_id();
    html! {
        span.heartbeat.tinted.(view.class) data-hint=(hint) role="img" aria-label="Heartbeat" {
            (icon(Icon::Heartbeat))
            (render_hint(&hint, view.hint.clone()))
        }
    }
}
