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

    /// A hung run or a failure fires at once; a late start waits for the
    /// alert's `for`.
    pub fn judge(&mut self, job: &AppJob, at: Timestamp) -> Observation {
        if let Some(run) = self.running {
            // A run that's still going when the next one is due counts for it.
            if self.next_due.is_some_and(|due| due <= at) {
                self.next_due = job.schedule.next_after(at);
            }
            if run.deadline.is_some_and(|deadline| deadline <= at) {
                return Observation::Urgent(format!(
                    "hung, running since {}",
                    format_local(run.started)
                ));
            }
        }
        if let Some(last) = self.last
            && !last.ok
        {
            let ran = Dur::from(Duration::from_secs(u64::from(
                last.finished.saturating_sub(last.started),
            )));
            return Observation::Urgent(format!(
                "failed at {} after {ran}",
                format_local(last.finished)
            ));
        }
        match self.next_due {
            Some(due) if self.running.is_none() && due <= at => {
                Observation::Problem(format!("didn't start, due at {}", format_local(due)))
            }
            _ => Observation::Fine,
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

    fn is_urgent(o: &Observation) -> bool {
        matches!(o, Observation::Urgent(_))
    }

    #[test]
    fn a_run_on_time_is_fine() {
        let job = job(None);
        let mut state = JobState::new(&job, T0 - MINUTE);
        assert_eq!(state.next_due, Some(T0));
        state.report(&job, Event::Start, T0 + 5).unwrap();
        assert_eq!(state.next_due, Some(T0 + 10 * MINUTE));
        assert_eq!(state.judge(&job, T0 + 5 * MINUTE), Observation::Fine);
        state.report(&job, Event::Ok, T0 + 6 * MINUTE).unwrap();
        assert_eq!(state.judge(&job, T0 + 9 * MINUTE), Observation::Fine);
    }

    #[test]
    fn a_missed_start_is_a_problem_until_it_starts() {
        let job = job(None);
        let mut state = JobState::new(&job, T0 - MINUTE);
        assert_eq!(state.judge(&job, T0 - 1), Observation::Fine);
        assert!(matches!(state.judge(&job, T0), Observation::Problem(_)));
        assert!(matches!(
            state.judge(&job, T0 + 30 * MINUTE),
            Observation::Problem(_)
        ));
        state.report(&job, Event::Start, T0 + 31 * MINUTE).unwrap();
        assert_eq!(state.judge(&job, T0 + 32 * MINUTE), Observation::Fine);
    }

    #[test]
    fn a_run_past_its_deadline_is_hung() {
        let job_until_next = job(None);
        let mut state = JobState::new(&job_until_next, T0 - MINUTE);
        state.report(&job_until_next, Event::Start, T0).unwrap();
        assert!(is_urgent(&state.judge(&job_until_next, T0 + 10 * MINUTE)));

        let job_with_max = job(Some(Duration::from_secs(25 * 60)));
        let mut state = JobState::new(&job_with_max, T0 - MINUTE);
        state.report(&job_with_max, Event::Start, T0).unwrap();
        // Due times that pass while it runs aren't missed.
        assert_eq!(
            state.judge(&job_with_max, T0 + 20 * MINUTE),
            Observation::Fine
        );
        assert!(is_urgent(&state.judge(&job_with_max, T0 + 25 * MINUTE)));
        state
            .report(&job_with_max, Event::Ok, T0 + 26 * MINUTE)
            .unwrap();
        assert_eq!(state.next_due, Some(T0 + 30 * MINUTE));
        assert_eq!(
            state.judge(&job_with_max, T0 + 27 * MINUTE),
            Observation::Fine
        );
    }

    #[test]
    fn a_failure_lasts_until_a_run_succeeds() {
        let job = job(None);
        let mut state = JobState::new(&job, T0 - MINUTE);
        state.report(&job, Event::Start, T0).unwrap();
        state.report(&job, Event::Fail, T0 + MINUTE).unwrap();
        assert!(is_urgent(&state.judge(&job, T0 + 2 * MINUTE)));
        state.report(&job, Event::Start, T0 + 10 * MINUTE).unwrap();
        assert!(is_urgent(&state.judge(&job, T0 + 11 * MINUTE)));
        state.report(&job, Event::Ok, T0 + 12 * MINUTE).unwrap();
        assert_eq!(state.judge(&job, T0 + 13 * MINUTE), Observation::Fine);
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
