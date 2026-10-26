use maud::{Markup, html};

use crate::config::{App, AppCommand};
use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp};
use crate::store::{Entity, Store};

use super::super::super::ui::{Icon, Level, ago, status_for, tinted_icon};

pub struct CommandRow {
    pub(super) name: &'static str,
    pub(super) status: Option<Status>,
    pub(super) running_since: Option<Timestamp>,
    /// How long the last finished run took.
    pub(super) took: Option<Sample>,
    /// What the last successful run printed.
    pub(super) message: Option<String>,
    pub(super) error: Option<String>,
}

pub fn command_row(store: &Store, app: &App, command: &'static AppCommand) -> CommandRow {
    let entity = store.entity(&EntityId::Command {
        app: app.id.clone(),
        command: command.name.clone(),
    });
    CommandRow {
        name: &command.name,
        status: entity.and_then(Entity::status),
        running_since: entity.and_then(Entity::running_since),
        took: entity
            .and_then(|e| e.series(MetricKind::Runtime))
            .and_then(|s| s.latest()),
        message: entity.and_then(Entity::message).map(str::to_owned),
        error: entity.and_then(Entity::error).map(str::to_owned),
    }
}

impl CommandRow {
    fn level(&self) -> Level {
        match self.status {
            None => Level::Unknown,
            Some(Status::Down { .. }) => Level::Problem,
            Some(_) => Level::Ok,
        }
    }
}

/// The status icon, name and state, how long the last run took and what it
/// printed or why it failed, then the run in progress.
pub fn render_command_item(row: &CommandRow) -> Markup {
    let level = row.level();
    html! {
        li {
            (tinted_icon(Icon::Command, level.class(), None))
            " "
            span.highlight.(level.text_class()) { (row.name) }
            " — " (status_for(row.status))
            @if let Some(took) = row.took {
                ", took " (seconds(took.value))
            }
            @if let Some(since) = row.running_since {
                ", running for " (ago(since))
            }
            @match (row.status, &row.message, &row.error) {
                (Some(Status::Down { .. }), _, Some(error)) => {
                    span.command-output { (error) }
                }
                (Some(Status::Up { .. }), Some(message), _) if !message.is_empty() => {
                    span.command-output { (message) }
                }
                _ => {}
            }
        }
    }
}

fn seconds(value: f32) -> String {
    match value {
        ..10.0 => format!("{value:.1}s"),
        _ => format!("{value:.0}s"),
    }
}
