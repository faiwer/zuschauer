use maud::{Markup, html};

use crate::config::Heartbeat;
use crate::heartbeat::Status;

use super::super::ui::ago;

/// The header's heartbeat indicator.
pub struct HeartbeatDot {
    /// A `.tint-*` class.
    class: &'static str,
    title: String,
}

pub fn heartbeat_dot(heartbeat: &Heartbeat, status: &Status) -> HeartbeatDot {
    let (class, state) = match (&status.failing, status.sent) {
        (Some((since, error)), sent) => {
            let sent = sent
                .map(|at| format!(", last sent {} ago", ago(at)))
                .unwrap_or_default();
            (
                "tint-problem",
                format!("failing for {}{sent}: {error}", ago(*since)),
            )
        }
        (None, Some(at)) => ("tint-ok", format!("sent {} ago", ago(at))),
        (None, None) => ("tint-unknown", "pending".to_owned()),
    };
    HeartbeatDot {
        class,
        title: format!("Heartbeat: {heartbeat}, {state}"),
    }
}

pub fn render_heartbeat_dot(dot: &HeartbeatDot) -> Markup {
    html! {
        span.heartbeat-dot.(dot.class) title=(dot.title) role="img" aria-label=(dot.title) {}
    }
}
