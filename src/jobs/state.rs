use std::time::Duration;

use crate::alerts::Observation;
use crate::config::{AppJob, Dur};
use crate::cron::format_local;
use crate::model::Timestamp;

/// What a job reports about a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Start,
    Ok,
    Fail,
}

impl Event {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "start" => Some(Self::Start),
            "ok" => Some(Self::Ok),
            "fail" => Some(Self::Fail),
            _ => None,
        }
    }
}

/// `ok` or `fail` came without a `start`.
#[derive(Debug, PartialEq, Eq)]
pub struct NoRun;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub started: Timestamp,
    /// Hung after this. `None` when the schedule never comes again.
    deadline: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finished {
    pub started: Timestamp,
    pub finished: Timestamp,
    pub ok: bool,
}

/// What a job's state means right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Between runs, the last one fine or none seen yet.
    Waiting,
    Running {
        since: Timestamp,
    },
    /// `due` passed without a `start`.
    Late {
        due: Timestamp,
    },
    /// Running past its deadline.
    Hung {
        since: Timestamp,
    },
    /// The last finished run failed. `retrying` is when the run going now
    /// started, if one is.
    Failed {
        run: Finished,
        retrying: Option<Timestamp>,
    },
}

impl Verdict {
    /// A hung run or a failure fires at once; a late start waits for the
    /// alert's `for`.
    pub fn observation(self) -> Observation {
        match self {
            Self::Waiting | Self::Running { .. } => Observation::Fine,
            Self::Late { due } => {
                Observation::Problem(format!("didn't start, due at {}", format_local(due)))
            }
            Self::Hung { since } => {
                Observation::Urgent(format!("hung, running since {}", format_local(since)))
            }
            Self::Failed { run, .. } => {
                let took = Dur::from(Duration::from_secs(u64::from(
                    run.finished.saturating_sub(run.started),
                )));
                Observation::Urgent(format!(
                    "failed at {} after {took}",
                    format_local(run.finished)
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobState {
    /// `None` when the schedule never comes again.
    pub next_due: Option<Timestamp>,
    pub running: Option<Run>,
    pub last: Option<Finished>,
}

impl JobState {
    /// Nothing is known about runs before `at`, so the first due time is the
    /// next one.
    pub fn new(job: &AppJob, at: Timestamp) -> Self {
        Self {
            next_due: job.schedule.next_after(at),
            running: None,
            last: None,
        }
    }

    /// A `start` while a run is open replaces it: the old one was lost.
    pub fn report(&mut self, job: &AppJob, event: Event, at: Timestamp) -> Result<(), NoRun> {
        match event {
            Event::Start => {
                self.next_due = job.schedule.next_after(at);
                let deadline = match job.max_duration {
                    Some(max) => Some(at.saturating_add(seconds(max))),
                    None => self.next_due,
                };
                self.running = Some(Run {
                    started: at,
                    deadline,
                });
            }
            Event::Ok | Event::Fail => {
                let run = self.running.take().ok_or(NoRun)?;
                self.last = Some(Finished {
                    started: run.started,
                    finished: at,
                    ok: event == Event::Ok,
                });
            }
        }
        Ok(())
    }

    /// Hung wins over failed, and failed over late: a failure stays the
    /// reason until a run succeeds, even while it's retried or missed.
    pub fn judge(&mut self, job: &AppJob, at: Timestamp) -> Verdict {
        if let Some(run) = self.running {
            // A run that's still going when the next one is due counts for it.
            if self.next_due.is_some_and(|due| due <= at) {
                self.next_due = job.schedule.next_after(at);
            }
            if run.deadline.is_some_and(|deadline| deadline <= at) {
                return Verdict::Hung { since: run.started };
            }
        }
        if let Some(last) = self.last
            && !last.ok
        {
            return Verdict::Failed {
                run: last,
                retrying: self.running.map(|run| run.started),
            };
        }
        match (self.running, self.next_due) {
            (Some(run), _) => Verdict::Running { since: run.started },
            (None, Some(due)) if due <= at => Verdict::Late { due },
            _ => Verdict::Waiting,
        }
    }

    /// When `judge` could next say something else, if no report comes first.
    /// Call it after `judge`, which moves `next_due` past a running job.
    pub fn next_change(&self, at: Timestamp) -> Option<Timestamp> {
        let candidates = match self.running {
            Some(run) => [run.deadline, self.next_due],
            None => [self.next_due, None],
        };
        candidates.into_iter().flatten().filter(|&t| t > at).min()
    }
}

pub(super) fn seconds(duration: Duration) -> Timestamp {
    Timestamp::try_from(duration.as_secs()).unwrap_or(Timestamp::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cron::Schedule;

    const MINUTE: Timestamp = 60;

    /// Every 10 minutes, so due times are multiples of 600 in any time zone
    /// with whole-hour or half-hour offsets.
    fn job(max_duration: Option<Duration>) -> AppJob {
        AppJob {
            name: "test".to_owned(),
            token: "0123456789abcdef".to_owned(),
            schedule: Schedule::parse("*/10 * * * *").unwrap(),
            max_duration,
            alert: None,
        }
    }

    const T0: Timestamp = 1_800_000_000; // a multiple of 600

    #[test]
    fn a_run_on_time_is_fine() {
        let job = job(None);
        let mut state = JobState::new(&job, T0 - MINUTE);
        assert_eq!(state.next_due, Some(T0));
        state.report(&job, Event::Start, T0 + 5).unwrap();
        assert_eq!(state.next_due, Some(T0 + 10 * MINUTE));
        assert_eq!(
            state.judge(&job, T0 + 5 * MINUTE),
            Verdict::Running { since: T0 + 5 }
        );
        state.report(&job, Event::Ok, T0 + 6 * MINUTE).unwrap();
        assert_eq!(state.judge(&job, T0 + 9 * MINUTE), Verdict::Waiting);
    }

    #[test]
    fn a_missed_start_is_late_until_it_starts() {
        let job = job(None);
        let mut state = JobState::new(&job, T0 - MINUTE);
        assert_eq!(state.judge(&job, T0 - 1), Verdict::Waiting);
        let late = Verdict::Late { due: T0 };
        assert_eq!(state.judge(&job, T0), late);
        assert_eq!(state.judge(&job, T0 + 30 * MINUTE), late);
        assert!(matches!(late.observation(), Observation::Problem(_)));
        state.report(&job, Event::Start, T0 + 31 * MINUTE).unwrap();
        assert_eq!(
            state.judge(&job, T0 + 32 * MINUTE),
            Verdict::Running {
                since: T0 + 31 * MINUTE
            }
        );
    }

    #[test]
    fn a_run_past_its_deadline_is_hung() {
        let job_until_next = job(None);
        let mut state = JobState::new(&job_until_next, T0 - MINUTE);
        state.report(&job_until_next, Event::Start, T0).unwrap();
        let hung = Verdict::Hung { since: T0 };
        assert_eq!(state.judge(&job_until_next, T0 + 10 * MINUTE), hung);
        assert!(matches!(hung.observation(), Observation::Urgent(_)));

        let job_with_max = job(Some(Duration::from_secs(25 * 60)));
        let mut state = JobState::new(&job_with_max, T0 - MINUTE);
        state.report(&job_with_max, Event::Start, T0).unwrap();
        // Due times that pass while it runs aren't missed.
        assert_eq!(
            state.judge(&job_with_max, T0 + 20 * MINUTE),
            Verdict::Running { since: T0 }
        );
        assert_eq!(state.judge(&job_with_max, T0 + 25 * MINUTE), hung);
        state
            .report(&job_with_max, Event::Ok, T0 + 26 * MINUTE)
            .unwrap();
        assert_eq!(state.next_due, Some(T0 + 30 * MINUTE));
        assert_eq!(
            state.judge(&job_with_max, T0 + 27 * MINUTE),
            Verdict::Waiting
        );
    }

    #[test]
    fn a_failure_lasts_until_a_run_succeeds() {
        let job = job(None);
        let mut state = JobState::new(&job, T0 - MINUTE);
        state.report(&job, Event::Start, T0).unwrap();
        state.report(&job, Event::Fail, T0 + MINUTE).unwrap();
        let run = Finished {
            started: T0,
            finished: T0 + MINUTE,
            ok: false,
        };
        let failed = Verdict::Failed {
            run,
            retrying: None,
        };
        assert_eq!(state.judge(&job, T0 + 2 * MINUTE), failed);
        assert!(matches!(failed.observation(), Observation::Urgent(_)));
        // Missing the next start doesn't make it merely late.
        assert_eq!(state.judge(&job, T0 + 15 * MINUTE), failed);
        state.report(&job, Event::Start, T0 + 16 * MINUTE).unwrap();
        assert_eq!(
            state.judge(&job, T0 + 17 * MINUTE),
            Verdict::Failed {
                run,
                retrying: Some(T0 + 16 * MINUTE)
            }
        );
        state.report(&job, Event::Ok, T0 + 18 * MINUTE).unwrap();
        assert_eq!(state.judge(&job, T0 + 19 * MINUTE), Verdict::Waiting);
    }

    #[test]
    fn knows_when_to_look_again() {
        let job = job(Some(Duration::from_secs(25 * 60)));
        let mut state = JobState::new(&job, T0 - MINUTE);
        assert_eq!(state.next_change(T0 - MINUTE), Some(T0));
        // Late: nothing changes until it starts.
        state.judge(&job, T0);
        assert_eq!(state.next_change(T0), None);
        state.report(&job, Event::Start, T0 + MINUTE).unwrap();
        state.judge(&job, T0 + MINUTE);
        assert_eq!(state.next_change(T0 + MINUTE), Some(T0 + 10 * MINUTE));
        state.judge(&job, T0 + 20 * MINUTE);
        assert_eq!(state.next_change(T0 + 20 * MINUTE), Some(T0 + 26 * MINUTE));
    }

    #[test]
    fn a_finish_needs_a_start() {
        let job = job(None);
        let mut state = JobState::new(&job, T0);
        assert_eq!(state.report(&job, Event::Ok, T0), Err(NoRun));
        state.report(&job, Event::Start, T0).unwrap();
        state.report(&job, Event::Ok, T0).unwrap();
        assert_eq!(state.report(&job, Event::Fail, T0), Err(NoRun));
    }
}
