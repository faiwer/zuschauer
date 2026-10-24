use maud::{Markup, html};

use crate::config::Config;
use crate::heartbeat;
use crate::model::{Timestamp, now};

use super::super::ui::{Icon, Level, date_time, icon, render_logo, tinted_icon};
use super::apps::{app_containers, app_level, app_views};
use super::containers::ContainerRow;
use super::heartbeat::{HeartbeatDot, heartbeat_dot, render_heartbeat_dot};

pub struct HeaderView {
    updated: Timestamp,
    health: Vec<Health>,
    /// `None` without a configured heartbeat.
    heartbeat: Option<HeartbeatDot>,
}

/// How many of one kind of thing are fine, and the worst of them.
struct Health {
    icon: Icon,
    label: &'static str,
    up: usize,
    total: usize,
    worst: Level,
}

pub fn header_view(config: &'static Config) -> HeaderView {
    let apps = app_views(config);
    let containers = apps.iter().flat_map(app_containers);
    HeaderView {
        updated: now(),
        health: [
            health(Icon::App, "Apps", apps.iter().map(app_level)),
            health(
                Icon::Container,
                "Containers",
                containers.map(ContainerRow::level),
            ),
        ]
        .into_iter()
        .filter(|h| h.total > 0)
        .collect(),
        heartbeat: config
            .heartbeat
            .as_ref()
            .map(|heartbeat| heartbeat_dot(heartbeat, &heartbeat::status())),
    }
}

fn health(icon: Icon, label: &'static str, levels: impl Iterator<Item = Level>) -> Health {
    let mut health = Health {
        icon,
        label,
        up: 0,
        total: 0,
        worst: Level::Unknown,
    };
    for level in levels {
        health.total += 1;
        health.up += usize::from(level == Level::Ok);
        health.worst = health.worst.max(level);
    }
    health
}

pub fn render_header(view: &HeaderView) -> Markup {
    html! {
        header.site-header {
            div.site-brand {
                span.site-icon { (render_logo()) }
                div.site-title {
                    h1 {
                        "Zuschauer"
                        @if let Some(dot) = &view.heartbeat { (render_heartbeat_dot(dot)) }
                    }
                    p { "Infrastructure monitoring" }
                }
            }
            (render_updated(view.updated))
            @for health in &view.health {
                (render_health(health))
            }
        }
    }
}

/// The reload button beside when the page was rendered. A plain GET form, so
/// reloading needs no script.
fn render_updated(at: Timestamp) -> Markup {
    html! {
        div.updated {
            form method="get" {
                button.icon-button type="submit" title="Reload" aria-label="Reload" {
                    (icon(Icon::Reload))
                }
            }
            div.updated-lines {
                span { "Last updated" }
                span { (date_time(at)) }
            }
        }
    }
}

/// Tinted only when something is wrong.
fn render_health(health: &Health) -> Markup {
    let bad = health.worst.is_bad();
    let tint = health.worst.class();
    let stat_icon = if bad {
        tinted_icon(health.icon, tint)
    } else {
        icon(health.icon)
    };
    render_stat(
        &stat_icon,
        html! {
            span.stat-value {
                span.(health.worst.text_class()) { (health.up) }
                " / " (health.total)
            }
            span.stat-label { (health.label) }
        },
    )
}

/// An `.icon` beside two lines.
fn render_stat(icon: &Markup, lines: Markup) -> Markup {
    html! {
        div.stat {
            (icon)
            div.stat-lines { (lines) }
        }
    }
}
