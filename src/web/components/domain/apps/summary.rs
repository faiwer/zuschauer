use crate::jobs::Verdict;
use crate::model::{Status, now};

use super::super::super::ui::{Level, until};
use super::super::containers::ContainerRow;
use super::app::AppView;

/// A level with what explains it.
pub struct Verdicts {
    pub level: Level,
    pub notes: Vec<String>,
}

impl Verdicts {
    fn new() -> Self {
        Self {
            level: Level::Unknown,
            notes: Vec::new(),
        }
    }

    fn add(&mut self, level: Level, note: Option<String>) {
        self.level = self.level.max(level);
        self.notes.extend(note);
    }
}

pub struct AppSummary {
    /// Checks, containers, services, commands and jobs; certificates have
    /// their own.
    pub health: Verdicts,
    /// `None` without a running container that reported it.
    pub cpu: Option<f32>,
    pub memory_bytes: Option<f32>,
    /// `None` without certificates.
    pub certificates: Option<Verdicts>,
}

pub fn summarize(app: &AppView) -> AppSummary {
    AppSummary {
        health: health(app),
        cpu: sum(app, |c| c.cpu.map(|s| s.value)),
        memory_bytes: sum(app, |c| c.memory.map(|s| s.value)),
        certificates: certificates(app),
    }
}

fn health(app: &AppView) -> Verdicts {
    let mut health = Verdicts::new();
    for check in &app.checks {
        match check.status {
            Some(Status::Down { .. }) => {
                health.add(Level::Problem, Some(format!("{} down", check.name)))
            }
            Some(_) => health.add(Level::Ok, None),
            None => {}
        }
    }
    for container in &app.containers {
        match container.status {
            Some(Status::Running) => health.add(Level::Ok, None),
            Some(Status::Paused) => {
                health.add(Level::Warning, Some(format!("{} paused", container.name)))
            }
            Some(_) => health.add(Level::Problem, Some(format!("{} gone", container.name))),
            None => {}
        }
    }
    for service in &app.services {
        match service.status {
            Some(Status::Down { .. }) => {
                health.add(Level::Problem, Some(format!("{} down", service.name)))
            }
            Some(_) => health.add(Level::Ok, None),
            None => {}
        }
    }
    for command in &app.commands {
        match command.status {
            Some(Status::Down { .. }) => {
                health.add(Level::Problem, Some(format!("{} failed", command.name)))
            }
            Some(_) => health.add(Level::Ok, None),
            None => {}
        }
    }
    for job in &app.jobs {
        match job.state.as_ref().map(|(_, verdict)| verdict) {
            Some(Verdict::Failed { .. }) => {
                health.add(Level::Problem, Some(format!("{} failed", job.name)))
            }
            Some(Verdict::Hung { .. }) => {
                health.add(Level::Problem, Some(format!("{} hung", job.name)))
            }
            Some(Verdict::Late { .. }) => {
                health.add(Level::Warning, Some(format!("{} late", job.name)))
            }
            Some(_) => health.add(Level::Ok, None),
            None => {}
        }
    }
    health
}

fn sum(app: &AppView, value: impl Fn(&ContainerRow) -> Option<f32>) -> Option<f32> {
    counted_containers(app)
        .filter_map(value)
        .reduce(|a, b| a + b)
}

/// The containers whose usage counts: still running or paused.
pub fn counted_containers(app: &AppView) -> impl Iterator<Item = &ContainerRow> {
    app.containers
        .iter()
        .filter(|c| matches!(c.status, Some(Status::Running | Status::Paused)))
}

fn certificates(app: &AppView) -> Option<Verdicts> {
    if app.certificates.is_empty() {
        return None;
    }
    let now = now();
    let mut verdicts = Verdicts::new();
    for row in &app.certificates {
        let (level, note) = match (row.status, row.expires_at) {
            (_, Some(at)) if at <= now => (Level::Problem, "expired".to_owned()),
            (Some(Status::Down { .. }), _) => (Level::Warning, "can't be checked".to_owned()),
            (_, Some(at)) if row.expiring => (Level::Warning, format!("expires in {}", until(at))),
            (_, Some(at)) => (Level::Ok, format!("valid, expires in {}", until(at))),
            _ => (Level::Unknown, "pending".to_owned()),
        };
        verdicts.add(level, Some(format!("{}: {note}", row.host)));
    }
    Some(verdicts)
}
