use std::future::pending;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::sleep;

use crate::alerts::{Observation, observe};
use crate::config::{App, AppJob};
use crate::model::{Timestamp, now};

use super::state::{Event, JobState, NoRun, seconds};

struct Tracked {
    app: &'static App,
    job: &'static AppJob,
    state: Mutex<JobState>,
    /// Wakes the job's task after a report.
    reported: Notify,
}

impl Tracked {
    fn state(&self) -> MutexGuard<'_, JobState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

static JOBS: OnceLock<Vec<Tracked>> = OnceLock::new();

fn find(app: &str, job: &str) -> Option<&'static Tracked> {
    JOBS.get()?
        .iter()
        .find(|t| t.app.id == app && t.job.name == job)
}

/// Starts tracking every app's jobs, with a task per job that sleeps until
/// the job's state can next change.
pub fn spawn(apps: &'static [App]) {
    let at = now();
    let tracked = apps
        .iter()
        .flat_map(|app| {
            app.jobs.iter().map(move |job| Tracked {
                app,
                job,
                state: Mutex::new(JobState::new(job, at)),
                reported: Notify::new(),
            })
        })
        .collect();
    for tracked in JOBS.get_or_init(|| tracked) {
        tokio::spawn(watch(tracked));
    }
}

async fn watch(tracked: &'static Tracked) {
    // When this task first saw the current problem.
    let mut problem_since: Option<Timestamp> = None;
    loop {
        let at = now();
        let (observation, mut wake) = {
            let mut state = tracked.state();
            let observation = state.judge(tracked.job, at).observation();
            (observation, state.next_change(at))
        };
        if let Some(alert) = &tracked.job.alert {
            if matches!(observation, Observation::Problem(_)) {
                let since = *problem_since.get_or_insert(at);
                // The alert fires only when observed after `for`, and
                // `observe` takes its own time, which can be a second later.
                let fires_at = since.saturating_add(seconds(alert.after) + 1);
                if fires_at > at {
                    wake = Some(wake.map_or(fires_at, |w| w.min(fires_at)));
                }
            } else {
                problem_since = None;
            }
            observe(alert, observation);
        }
        let timer = async {
            match wake {
                Some(wake) => {
                    let left = wake.saturating_sub(now());
                    sleep(Duration::from_secs(left.into())).await;
                }
                None => pending().await,
            }
        };
        tokio::select! {
            () = timer => {}
            () = tracked.reported.notified() => {}
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReportError {
    /// No such job, or no such event.
    NotFound,
    /// The token is missing or wrong.
    Unauthorized,
    /// `ok` or `fail` without a `start`.
    NoRun,
}

/// Records what the job `app/job` reported. `token` is what the request
/// carried.
pub fn report(app: &str, job: &str, event: &str, token: Option<&str>) -> Result<(), ReportError> {
    let event = Event::parse(event).ok_or(ReportError::NotFound)?;
    let tracked = find(app, job).ok_or(ReportError::NotFound)?;
    if token != Some(tracked.job.token.as_str()) {
        return Err(ReportError::Unauthorized);
    }
    tracked
        .state()
        .report(tracked.job, event, now())
        .map_err(|NoRun| ReportError::NoRun)?;
    tracked.reported.notify_one();
    Ok(())
}

/// The state of the job `app/job`, if it's tracked.
pub fn state(app: &str, job: &str) -> Option<JobState> {
    find(app, job).map(|t| t.state().clone())
}
