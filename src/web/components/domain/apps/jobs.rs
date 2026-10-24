use maud::{Markup, html};

use crate::config::{App, AppJob};
use crate::cron::format_local;
use crate::jobs::{self, JobState, Verdict};
use crate::model::now;

use super::super::super::ui::{ago, span};

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

pub fn render_job_row(row: &JobRow) -> Markup {
    html! {
        tr.table-row {
            td.highlight { (row.name) }
            @if let Some((state, verdict)) = &row.state {
                (render_job_cells(state, *verdict))
            } @else {
                td colspan="3" { "—" }
            }
        }
    }
}

fn render_job_cells(state: &JobState, verdict: Verdict) -> Markup {
    html! {
        td {
            @match verdict {
                Verdict::Hung { since } => { strong { "hung" } ", running " (ago(since)) }
                Verdict::Failed { retrying: Some(since), .. } => {
                    strong { "failed" } ", retry running " (ago(since))
                }
                Verdict::Failed { retrying: None, .. } => strong { "failed" },
                Verdict::Late { .. } => strong { "late" },
                Verdict::Running { since } => { "running " (ago(since)) }
                Verdict::Waiting if state.last.is_some() => "ok",
                Verdict::Waiting => "waiting",
            }
        }
        td {
            @match state.last {
                Some(last) => {
                    (if last.ok { "succeeded " } else { "failed " }) (ago(last.finished)) " ago, took "
                    (span(last.finished.saturating_sub(last.started)))
                }
                None => "no runs yet",
            }
        }
        td {
            @match state.next_due {
                Some(due) => { "due " (format_local(due)) }
                None => "never due",
            }
        }
    }
}
