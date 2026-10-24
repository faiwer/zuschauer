use maud::{Markup, html};

use crate::config::Heartbeat;
use crate::heartbeat::Status;

use super::format::ago;

pub fn render_heartbeat(heartbeat: &Heartbeat, status: &Status) -> Markup {
    html! {
        h1 { "Heartbeat" }
        table {
            tr {
                th { (heartbeat.to_string()) }
                (render_heartbeat_cells(status))
            }
        }
    }
}

fn render_heartbeat_cells(status: &Status) -> Markup {
    html! {
        @match &status.failing {
            Some((since, error)) => {
                td {
                    strong { "failing" } " for " (ago(*since))
                    @if let Some(at) = status.sent { ", last sent " (ago(at)) " ago" }
                }
                td { (error) }
            }
            None => td colspan="2" {
                @match status.sent {
                    Some(at) => { "sent " (ago(at)) " ago" }
                    None => "pending",
                }
            },
        }
    }
}
