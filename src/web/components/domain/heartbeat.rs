use maud::{Markup, html};

use crate::config::Heartbeat;
use crate::heartbeat::Status;

use super::super::ui::{ago, hint_id, render_hint};

/// The header's heartbeat indicator.
pub struct HeartbeatDot {
    /// A `.tint-*` class.
    class: &'static str,
    hint: Markup,
}

pub fn heartbeat_dot(heartbeat: &Heartbeat, status: &Status) -> HeartbeatDot {
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
    HeartbeatDot {
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

pub fn render_heartbeat_dot(dot: &HeartbeatDot) -> Markup {
    let hint = hint_id();
    html! {
        span.heartbeat-dot.(dot.class) data-hint=(hint) role="img" aria-label="Heartbeat" {
            (render_hint(&hint, dot.hint.clone()))
        }
    }
}
