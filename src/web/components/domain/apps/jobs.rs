use maud::{Markup, html};

use crate::config::{App, AppJob};
use crate::jobs::{self, Finished, JobState, Verdict};
use crate::model::now;

use super::super::super::ui::{Icon, Level, ago, date_time, span, tinted_icon};

pub struct JobRow {
    pub(super) name: &'static str,
    pub(super) state: Option<(JobState, Verdict)>,
}

pub fn job_row(app: &App, job: &'static AppJob) -> JobRow {
    JobRow {
        name: &job.name,
        state: jobs::state(&app.id, &job.name).map(|mut state| {
            let verdict = state.judge(job, now());
            (state, verdict)
        }),
    }
}

impl JobRow {
    fn level(&self) -> Level {
        match &self.state {
            None | Some((JobState { last: None, .. }, Verdict::Waiting)) => Level::Unknown,
            Some((_, Verdict::Waiting | Verdict::Running { .. })) => Level::Ok,
            Some((_, Verdict::Late { .. })) => Level::Warning,
            Some((_, Verdict::Hung { .. } | Verdict::Failed { .. })) => Level::Problem,
        }
    }
}

/// The status icon and name, then what the verdict says.
pub fn render_job_item(row: &JobRow) -> Markup {
    let level = row.level();
    html! {
        li {
            (tinted_icon(Icon::Service, level.class(), None))
            " "
            span.highlight.(level.text_class()) { (row.name) }
            " — "
            @match &row.state {
                None => "no data yet",
                Some((state, verdict)) => (job_state(state, *verdict)),
            }
        }
    }
}

fn job_state(state: &JobState, verdict: Verdict) -> String {
    match verdict {
        Verdict::Waiting => match (state.last, state.next_due) {
            (Some(last), next) => {
                let next = next
                    .map(|due| format!(", next at {}", date_time(due)))
                    .unwrap_or_default();
                format!(
                    "succeeded {} ago in {}{next}",
                    ago(last.finished),
                    took(last)
                )
            }
            (None, Some(due)) => format!("waiting, due at {}", date_time(due)),
            (None, None) => "waiting".to_owned(),
        },
        Verdict::Running { since } => format!("running for {}", ago(since)),
        Verdict::Late { due } => format!("didn't start, due at {}", date_time(due)),
        Verdict::Hung { since } => format!("hung, running for {}", ago(since)),
        Verdict::Failed { run, retrying } => {
            let retrying = retrying
                .map(|since| format!(", retrying for {}", ago(since)))
                .unwrap_or_default();
            format!(
                "failed {} ago after {}{retrying}",
                ago(run.finished),
                took(run)
            )
        }
    }
}

fn took(run: Finished) -> String {
    span(run.finished.saturating_sub(run.started))
}
