use std::fmt::Write;

use crate::collect::Parts;
use crate::config::{App, Config};
use crate::model::{EntityId, MetricKind, Status};
use crate::store::{Entity, Store, store};
use crate::web::{date, mib, status_for, until};

const NO_RESULT: &str = "no result yet";

/// One block per section, each a title and aligned rows. `width`: the
/// terminal's, if it is one.
pub fn render(config: &Config, parts: Parts, width: Option<usize>) -> String {
    let store = store();
    let mut blocks = Vec::new();
    if parts.host {
        blocks.push(("Host".to_owned(), host(config, &store)));
    }
    if parts.containers {
        blocks.push(("Containers".to_owned(), containers(config, &store)));
    }
    for app in &config.apps {
        let rows = app_rows(app, parts, &store);
        if !rows.is_empty() {
            blocks.push((app.name.clone(), rows));
        }
    }
    let mut out = String::new();
    for (title, rows) in blocks {
        if !out.is_empty() {
            out.push('\n');
        }
        writeln!(out, "{title}").unwrap();
        out.push_str(&table(&rows, width));
    }
    out
}

fn host(config: &Config, store: &Store) -> Vec<Vec<String>> {
    let latest = |id: &EntityId, metric: MetricKind, unit: &str| {
        let entity = store.entity(id);
        match entity
            .and_then(|e| e.series(metric))
            .and_then(|s| s.latest())
        {
            Some(sample) => format!("{:.1}{unit}", sample.value),
            None => entity
                .and_then(Entity::error)
                .unwrap_or(NO_RESULT)
                .to_owned(),
        }
    };
    let mut rows = vec![
        vec![
            "CPU".to_owned(),
            latest(&EntityId::Host, MetricKind::Cpu, "%"),
        ],
        vec![
            "Memory".to_owned(),
            latest(&EntityId::Host, MetricKind::Memory, "%"),
        ],
    ];
    for mount in &config.host.storage {
        let id = EntityId::Mount(mount.path.clone());
        rows.push(vec![
            mount.path.display().to_string(),
            latest(&id, MetricKind::Storage, "%"),
        ]);
    }
    for sensor in &config.host.temperature {
        let id = EntityId::Sensor(sensor.id.clone());
        rows.push(vec![
            sensor.name.clone(),
            latest(&id, MetricKind::Temperature, "C"),
        ]);
    }
    rows
}

fn containers(config: &Config, store: &Store) -> Vec<Vec<String>> {
    if config.docker.is_none() {
        return vec![vec![
            "not watched, the config has no \"docker\" section".to_owned(),
        ]];
    }
    let latest = |entity: &Entity, metric: MetricKind| {
        entity
            .series(metric)
            .and_then(|s| s.latest())
            .map(|s| s.value)
    };
    let rows: Vec<Vec<String>> = store
        .entities()
        .filter_map(|(id, entity)| match id {
            EntityId::Container(name) => Some(vec![
                name.clone(),
                status_for(entity.status()),
                latest(entity, MetricKind::Cpu)
                    .map(|cpu| format!("{cpu:.1}% CPU"))
                    .unwrap_or_default(),
                latest(entity, MetricKind::MemoryBytes)
                    .map(mib)
                    .unwrap_or_default(),
            ]),
            _ => None,
        })
        .collect();
    if rows.is_empty() {
        return vec![vec!["none found".to_owned()]];
    }
    rows
}

fn app_rows(app: &App, parts: Parts, store: &Store) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    if parts.checks {
        for check in &app.health_checks {
            let entity = store.entity(&EntityId::HealthCheck {
                app: app.id.clone(),
                check: check.name.clone(),
            });
            let state = match entity.and_then(Entity::status) {
                Some(Status::Up { .. }) => {
                    let latency = entity
                        .and_then(|e| e.series(MetricKind::Latency))
                        .and_then(|s| s.latest());
                    match latency {
                        Some(ms) => format!("up, {:.0} ms", ms.value),
                        None => "up".to_owned(),
                    }
                }
                Some(_) => down(entity),
                None => NO_RESULT.to_owned(),
            };
            rows.push(vec!["check".to_owned(), check.name.clone(), state]);
        }
    }
    if parts.certificates {
        for watched in &app.certificates {
            let host = watched.target.to_string();
            let entity = store.entity(&EntityId::Certificate {
                app: app.id.clone(),
                host: host.clone(),
            });
            let expires_at = entity.and_then(Entity::expires_at);
            let state = match (entity.and_then(Entity::status), expires_at) {
                (Some(Status::Up { .. }), Some(at)) => {
                    format!("expires at {}, in {}", date(at), until(at))
                }
                (Some(_), _) => down(entity),
                (None, _) => NO_RESULT.to_owned(),
            };
            rows.push(vec!["certificate".to_owned(), host, state]);
        }
    }
    if parts.services {
        for service in &app.services {
            let entity = store.entity(&EntityId::Service {
                app: app.id.clone(),
                service: service.name.clone(),
            });
            let state = match entity.and_then(Entity::status) {
                Some(Status::Up { .. }) => "running".to_owned(),
                Some(_) => down(entity),
                None => NO_RESULT.to_owned(),
            };
            rows.push(vec!["service".to_owned(), service.name.clone(), state]);
        }
    }
    if parts.commands {
        for command in &app.commands {
            let entity = store.entity(&EntityId::Command {
                app: app.id.clone(),
                command: command.name.clone(),
            });
            let state = match entity.and_then(Entity::status) {
                Some(Status::Up { .. }) => match entity.and_then(Entity::message) {
                    Some("") | None => "ok".to_owned(),
                    Some(message) => format!("ok, {message}"),
                },
                Some(_) => entity
                    .and_then(Entity::error)
                    .unwrap_or("failed")
                    .to_owned(),
                None => NO_RESULT.to_owned(),
            };
            rows.push(vec!["command".to_owned(), command.name.clone(), state]);
        }
    }
    rows
}

fn down(entity: Option<&Entity>) -> String {
    match entity.and_then(Entity::error) {
        Some(error) => format!("down, {error}"),
        None => "down".to_owned(),
    }
}

/// Indented, every column but the last padded to its widest cell. The last
/// one wraps to `width` under itself, and is joined into one line without a
/// width.
fn table(rows: &[Vec<String>], width: Option<usize>) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for row in rows {
        let Some((last, cells)) = row.split_last() else {
            continue;
        };
        let mut lead = String::from(GAP);
        for (column, cell) in cells.iter().enumerate() {
            write!(lead, "{cell:<width$}{GAP}", width = widths[column]).unwrap();
        }
        let indent = lead.chars().count();
        let room = width.map_or(usize::MAX, |width| {
            width.saturating_sub(indent).max(NARROWEST)
        });
        for (i, line) in wrap(last, room).iter().enumerate() {
            let lead = if i == 0 {
                lead.clone()
            } else {
                " ".repeat(indent)
            };
            writeln!(out, "{}", format!("{lead}{line}").trim_end()).unwrap();
        }
    }
    out
}

const GAP: &str = "  ";
/// Below this, wrapping would leave a word or two per line.
const NARROWEST: usize = 20;

/// Lines of at most `width` characters, broken at whitespace. A word longer
/// than that stays whole.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let len = line.chars().count();
        if len > 0 && len + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|&cell| cell.to_owned()).collect()
    }

    #[test]
    fn aligns_columns() {
        let rows = [
            row(&["CPU", "6.5%"]),
            row(&["/usr/home", "97.6%"]),
            row(&["web", "running", "", "12 MiB"]),
        ];
        assert_eq!(
            table(&rows, None),
            "  CPU        6.5%\n  /usr/home  97.6%\n  web        running    12 MiB\n"
        );
    }

    #[test]
    fn wraps_the_last_column_under_itself() {
        let rows = [
            row(&[
                "check",
                "api",
                "down, connection refused by the remote host",
            ]),
            row(&["service", "nginx", "running"]),
        ];
        assert_eq!(
            table(&rows, Some(40)),
            "  check    api    down, connection\n                  refused by the remote\n                  host\n  service  nginx  running\n"
        );
    }

    #[test]
    fn joins_lines_without_a_width() {
        let rows = [row(&["service", "systemctl: Failed\n  to connect\n"])];
        assert_eq!(
            table(&rows, None),
            "  service  systemctl: Failed to connect\n"
        );
    }

    #[test]
    fn wraps_at_whitespace() {
        assert_eq!(wrap("aa bb cc", 5), ["aa bb", "cc"]);
        assert_eq!(wrap("aaaaaaa bb", 5), ["aaaaaaa", "bb"]);
        assert_eq!(wrap("", 5), [""]);
    }
}
