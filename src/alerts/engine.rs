//! An alert is OK, pending (its problem hasn't lasted `after` yet, unless
//! it's urgent) or firing.
//! Only the switches to firing and back to OK are reported, so a problem
//! that clears while pending is never announced.

use std::collections::BTreeMap;

use crate::config::Alert;
use crate::model::Timestamp;

/// What a collector saw.
#[derive(Debug, PartialEq)]
pub enum Observation {
    /// A reading, checked against the alert's threshold.
    Value(f32),
    Fine,
    /// Down, gone, expiring: why it's a problem.
    Problem(String),
    /// A problem that's certain already, like too many restarts within an
    /// hour or a failed job, so it fires without waiting for `after`.
    Urgent(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Change {
    Fired { since: Timestamp, detail: String },
    Resolved { since: Timestamp },
}

#[derive(Debug, Clone)]
pub(super) struct Problem {
    pub since: Timestamp,
    pub detail: String,
}

struct State {
    alert: &'static Alert,
    problem: Option<Problem>,
    fired: bool,
}

pub(super) struct Engine {
    /// By alert ID.
    states: BTreeMap<&'static str, State>,
}

impl Engine {
    pub const fn new() -> Self {
        Self {
            states: BTreeMap::new(),
        }
    }

    pub fn observe(
        &mut self,
        alert: &'static Alert,
        observation: Observation,
        at: Timestamp,
    ) -> Option<Change> {
        let blocked = alert.parents.iter().any(|id| {
            self.states
                .get(id.as_str())
                .is_some_and(|parent| parent.problem.is_some())
        });
        let state = self.states.entry(&alert.id).or_insert(State {
            alert,
            problem: None,
            fired: false,
        });

        let is_urgent = matches!(observation, Observation::Urgent(_));
        let detail = match observation {
            Observation::Value(value) => alert
                .threshold
                .as_ref()
                .filter(|t| t.exceeded(value, state.problem.is_some()))
                .map(|t| format!("{value:.1}{unit}, above {}{unit}", t.above, unit = t.unit)),
            Observation::Fine => None,
            Observation::Problem(detail) | Observation::Urgent(detail) => Some(detail),
        };

        let Some(detail) = detail else {
            let problem = state.problem.take()?;
            let fired = std::mem::take(&mut state.fired);
            return fired.then_some(Change::Resolved {
                since: problem.since,
            });
        };
        let problem = state.problem.get_or_insert(Problem {
            since: at,
            detail: String::new(),
        });
        problem.detail = detail;
        let lasted = at.saturating_sub(problem.since);
        if state.fired || blocked || (!is_urgent && u64::from(lasted) < alert.after.as_secs()) {
            return None;
        }
        state.fired = true;
        Some(Change::Fired {
            since: problem.since,
            detail: problem.detail.clone(),
        })
    }

    pub fn firing(&self) -> impl Iterator<Item = (&'static Alert, &Problem)> {
        self.states
            .values()
            .filter(|s| s.fired)
            .filter_map(|s| Some((s.alert, s.problem.as_ref()?)))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::{Severity, Threshold};

    fn alert(
        id: &str,
        after: u64,
        threshold: Option<Threshold>,
        parents: &[&str],
    ) -> &'static Alert {
        Box::leak(Box::new(Alert {
            id: id.to_owned(),
            label: id.to_owned(),
            severity: Severity::Warning,
            after: Duration::from_secs(after),
            parents: parents.iter().map(|p| (*p).to_owned()).collect(),
            threshold,
            restarts: None,
        }))
    }

    fn above(above: f32, resolve_below: Option<f32>) -> Option<Threshold> {
        Some(Threshold {
            above,
            resolve_below,
            unit: "%",
        })
    }

    fn fired(since: Timestamp, detail: &str) -> Option<Change> {
        Some(Change::Fired {
            since,
            detail: detail.to_owned(),
        })
    }

    #[test]
    fn fires_after_the_problem_lasts_and_resolves_once() {
        let cpu = alert("cpu", 60, above(85.0, None), &[]);
        let mut engine = Engine::new();
        assert_eq!(engine.observe(cpu, Observation::Value(90.0), 0), None);
        assert_eq!(engine.observe(cpu, Observation::Value(95.0), 30), None);
        assert_eq!(
            engine.observe(cpu, Observation::Value(92.0), 60),
            fired(0, "92.0%, above 85%")
        );
        assert_eq!(engine.observe(cpu, Observation::Value(99.0), 90), None);
        assert_eq!(engine.firing().count(), 1);
        assert_eq!(
            engine.observe(cpu, Observation::Value(50.0), 120),
            Some(Change::Resolved { since: 0 })
        );
        assert_eq!(engine.observe(cpu, Observation::Value(50.0), 150), None);
        assert_eq!(engine.firing().count(), 0);
    }

    #[test]
    fn a_problem_clearing_while_pending_is_silent() {
        let check = alert("check", 60, None, &[]);
        let mut engine = Engine::new();
        let down = || Observation::Problem("down".to_owned());
        assert_eq!(engine.observe(check, down(), 0), None);
        assert_eq!(engine.observe(check, Observation::Fine, 30), None);
        // The streak restarts.
        assert_eq!(engine.observe(check, down(), 40), None);
        assert_eq!(engine.observe(check, down(), 90), None);
        assert_eq!(engine.observe(check, down(), 100), fired(40, "down"));
    }

    #[test]
    fn an_urgent_problem_skips_the_wait() {
        let service = alert("service", 60, None, &[]);
        let mut engine = Engine::new();
        let down = || Observation::Problem("down".to_owned());
        assert_eq!(engine.observe(service, down(), 0), None);
        // The streak continues, so `since` is the first failure.
        assert_eq!(
            engine.observe(service, Observation::Urgent("restarts".to_owned()), 10),
            fired(0, "restarts")
        );
        assert_eq!(engine.observe(service, down(), 20), None);
        assert!(engine.observe(service, Observation::Fine, 30).is_some());
    }

    #[test]
    fn resolves_only_below_the_hysteresis_level() {
        let temp = alert("temp", 0, above(90.0, Some(85.0)), &[]);
        let mut engine = Engine::new();
        assert_eq!(engine.observe(temp, Observation::Value(88.0), 0), None);
        assert!(engine.observe(temp, Observation::Value(91.0), 10).is_some());
        assert_eq!(engine.observe(temp, Observation::Value(87.0), 20), None);
        assert_eq!(engine.observe(temp, Observation::Value(85.0), 30), None);
        assert_eq!(
            engine.observe(temp, Observation::Value(84.9), 40),
            Some(Change::Resolved { since: 10 })
        );
    }

    #[test]
    fn a_child_waits_while_a_parent_has_a_problem() {
        let db = alert("db", 120, None, &[]);
        let api = alert("api", 0, None, &["db"]);
        let mut engine = Engine::new();
        let problem = || Observation::Problem("down".to_owned());

        // The parent is only pending, and that already holds the child back.
        assert_eq!(engine.observe(db, problem(), 0), None);
        assert_eq!(engine.observe(api, problem(), 1), None);
        assert!(engine.observe(db, problem(), 120).is_some());
        assert_eq!(engine.observe(api, problem(), 121), None);

        // The parent recovers, the child doesn't: now the child fires, its
        // problem dating back to its first failure.
        assert!(engine.observe(db, Observation::Fine, 200).is_some());
        assert_eq!(engine.observe(api, problem(), 201), fired(1, "down"));
    }
}
