//! An alert is OK, pending (its problem hasn't lasted `after` yet, unless
//! it's urgent) or firing.
//! Only the switches to firing and back to OK are reported, so a problem
//! that clears while pending is never announced. A firing alert is announced
//! again when its problem gets louder, from a warning to critical.
//! An alert stays pending while an ancestor has a problem, and fires as soon
//! as the last such ancestor clears, without waiting for its next observation.

use std::collections::BTreeMap;

use crate::config::{Alert, Severity};
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
    /// A problem announced as a warning at most, like a command exiting with
    /// one of its `warnOn` codes.
    Warning(String),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Change {
    Fired {
        since: Timestamp,
        detail: String,
        severity: Severity,
    },
    Resolved {
        since: Timestamp,
        /// The latest problem's.
        detail: String,
        /// The loudest announced.
        severity: Severity,
    },
}

#[derive(Debug, Clone)]
pub(super) struct Problem {
    pub since: Timestamp,
    pub detail: String,
    /// Of the latest observation.
    pub severity: Severity,
    /// The latest observation was `Urgent`.
    urgent: bool,
}

impl Problem {
    fn due(&self, alert: &Alert, at: Timestamp) -> bool {
        self.urgent || u64::from(at.saturating_sub(self.since)) >= alert.after.as_secs()
    }
}

struct State {
    alert: &'static Alert,
    problem: Option<Problem>,
    /// While firing: the loudest severity announced.
    announced: Option<Severity>,
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

    /// Returns `alert`'s own change first, then those of the alerts it was
    /// holding back, when its problem clears.
    pub fn observe(
        &mut self,
        alert: &'static Alert,
        observation: Observation,
        at: Timestamp,
    ) -> Vec<(&'static Alert, Change)> {
        let blocked = self.is_blocked(alert);
        let state = self.states.entry(&alert.id).or_insert(State {
            alert,
            problem: None,
            announced: None,
        });

        let is_urgent = matches!(observation, Observation::Urgent(_));
        let severity = match observation {
            Observation::Warning(_) => alert.severity.at_most_warning(),
            _ => alert.severity,
        };
        let detail = match observation {
            Observation::Value(value) => alert
                .threshold
                .as_ref()
                .filter(|t| t.exceeded(value, state.problem.is_some()))
                .map(|t| format!("{value:.1}{unit}, above {}{unit}", t.above, unit = t.unit)),
            Observation::Fine => None,
            Observation::Problem(detail)
            | Observation::Urgent(detail)
            | Observation::Warning(detail) => Some(detail),
        };

        let Some(detail) = detail else {
            let Some(problem) = state.problem.take() else {
                return Vec::new();
            };
            let resolved = state.announced.take().map(|severity| {
                (
                    alert,
                    Change::Resolved {
                        since: problem.since,
                        detail: problem.detail,
                        severity,
                    },
                )
            });
            return resolved
                .into_iter()
                .chain(self.release(alert, at))
                .collect();
        };
        let problem = state.problem.get_or_insert(Problem {
            since: at,
            detail: String::new(),
            severity,
            urgent: false,
        });
        problem.detail = detail;
        problem.severity = severity;
        problem.urgent = is_urgent;
        let louder = state
            .announced
            .is_none_or(|announced| severity.is_louder_than(announced));
        if !louder || blocked || !problem.due(alert, at) {
            return Vec::new();
        }
        state.announced = Some(severity);
        vec![(
            alert,
            Change::Fired {
                since: problem.since,
                detail: problem.detail.clone(),
                severity,
            },
        )]
    }

    /// Any ancestor with a problem holds an alert back, pending or firing.
    fn is_blocked(&self, alert: &Alert) -> bool {
        alert.parents.iter().any(|id| {
            self.states
                .get(id.as_str())
                .is_some_and(|parent| parent.problem.is_some())
        })
    }

    /// Fires the descendants of `parent`, whose problem just cleared, that
    /// waited only for it. A descendant that's still held back by another
    /// ancestor waits for that one.
    fn release(&mut self, parent: &Alert, at: Timestamp) -> Vec<(&'static Alert, Change)> {
        let ready: Vec<&'static Alert> = self
            .states
            .values()
            .filter(|s| s.announced.is_none() && s.alert.parents.contains(&parent.id))
            .filter(|s| s.problem.as_ref().is_some_and(|p| p.due(s.alert, at)))
            .map(|s| s.alert)
            .filter(|alert| !self.is_blocked(alert))
            .collect();
        let mut changes = Vec::new();
        for alert in ready {
            let Some(state) = self.states.get_mut(alert.id.as_str()) else {
                continue;
            };
            let Some(problem) = &state.problem else {
                continue;
            };
            changes.push((
                alert,
                Change::Fired {
                    since: problem.since,
                    detail: problem.detail.clone(),
                    severity: problem.severity,
                },
            ));
            state.announced = Some(problem.severity);
        }
        changes
    }

    pub fn firing(&self) -> impl Iterator<Item = (&'static Alert, &Problem)> {
        self.states
            .values()
            .filter(|s| s.announced.is_some())
            .filter_map(|s| Some((s.alert, s.problem.as_ref()?)))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::Threshold;

    fn alert(
        id: &str,
        after: u64,
        threshold: Option<Threshold>,
        parents: &[&str],
    ) -> &'static Alert {
        alert_with(id, after, threshold, parents, Severity::Warning)
    }

    fn alert_with(
        id: &str,
        after: u64,
        threshold: Option<Threshold>,
        parents: &[&str],
        severity: Severity,
    ) -> &'static Alert {
        Box::leak(Box::new(Alert {
            id: id.to_owned(),
            label: id.to_owned(),
            severity,
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
        fired_as(since, detail, Severity::Warning)
    }

    fn fired_as(since: Timestamp, detail: &str, severity: Severity) -> Option<Change> {
        Some(Change::Fired {
            since,
            detail: detail.to_owned(),
            severity,
        })
    }

    impl Engine {
        /// `observe` for a lone alert, which changes nothing but itself.
        fn observe_one(
            &mut self,
            alert: &'static Alert,
            observation: Observation,
            at: Timestamp,
        ) -> Option<Change> {
            let mut changes = self.observe(alert, observation, at);
            assert!(changes.len() <= 1, "{changes:?}");
            changes.pop().map(|(changed, change)| {
                assert_eq!(changed.id, alert.id);
                change
            })
        }
    }

    fn ids(changes: &[(&'static Alert, Change)]) -> Vec<&'static str> {
        changes.iter().map(|(alert, _)| alert.id.as_str()).collect()
    }

    #[test]
    fn fires_after_the_problem_lasts_and_resolves_once() {
        let cpu = alert("cpu", 60, above(85.0, None), &[]);
        let mut engine = Engine::new();
        assert_eq!(engine.observe_one(cpu, Observation::Value(90.0), 0), None);
        assert_eq!(engine.observe_one(cpu, Observation::Value(95.0), 30), None);
        assert_eq!(
            engine.observe_one(cpu, Observation::Value(92.0), 60),
            fired(0, "92.0%, above 85%")
        );
        assert_eq!(engine.observe_one(cpu, Observation::Value(99.0), 90), None);
        assert_eq!(engine.firing().count(), 1);
        assert!(matches!(
            engine.observe_one(cpu, Observation::Value(50.0), 120),
            Some(Change::Resolved { since: 0, .. })
        ));
        assert_eq!(engine.observe_one(cpu, Observation::Value(50.0), 150), None);
        assert_eq!(engine.firing().count(), 0);
    }

    #[test]
    fn a_problem_clearing_while_pending_is_silent() {
        let check = alert("check", 60, None, &[]);
        let mut engine = Engine::new();
        let down = || Observation::Problem("down".to_owned());
        assert_eq!(engine.observe_one(check, down(), 0), None);
        assert_eq!(engine.observe_one(check, Observation::Fine, 30), None);
        // The streak restarts.
        assert_eq!(engine.observe_one(check, down(), 40), None);
        assert_eq!(engine.observe_one(check, down(), 90), None);
        assert_eq!(engine.observe_one(check, down(), 100), fired(40, "down"));
    }

    #[test]
    fn an_urgent_problem_skips_the_wait() {
        let service = alert("service", 60, None, &[]);
        let mut engine = Engine::new();
        let down = || Observation::Problem("down".to_owned());
        assert_eq!(engine.observe_one(service, down(), 0), None);
        // The streak continues, so `since` is the first failure.
        assert_eq!(
            engine.observe_one(service, Observation::Urgent("restarts".to_owned()), 10),
            fired(0, "restarts")
        );
        assert_eq!(engine.observe_one(service, down(), 20), None);
        assert!(engine.observe_one(service, Observation::Fine, 30).is_some());
    }

    #[test]
    fn resolves_only_below_the_hysteresis_level() {
        let temp = alert("temp", 0, above(90.0, Some(85.0)), &[]);
        let mut engine = Engine::new();
        assert_eq!(engine.observe_one(temp, Observation::Value(88.0), 0), None);
        assert!(
            engine
                .observe_one(temp, Observation::Value(91.0), 10)
                .is_some()
        );
        assert_eq!(engine.observe_one(temp, Observation::Value(87.0), 20), None);
        assert_eq!(engine.observe_one(temp, Observation::Value(85.0), 30), None);
        assert!(matches!(
            engine.observe_one(temp, Observation::Value(84.9), 40),
            Some(Change::Resolved { since: 10, .. })
        ));
    }

    #[test]
    fn a_child_waits_while_a_parent_has_a_problem() {
        let db = alert("db", 120, None, &[]);
        let api = alert("api", 0, None, &["db"]);
        let mut engine = Engine::new();
        let problem = || Observation::Problem("down".to_owned());

        // The parent is only pending, and that already holds the child back.
        assert_eq!(engine.observe_one(db, problem(), 0), None);
        assert_eq!(engine.observe_one(api, problem(), 1), None);
        assert!(engine.observe_one(db, problem(), 120).is_some());
        assert_eq!(engine.observe_one(api, problem(), 121), None);

        // The parent recovers, the child doesn't: the child fires right
        // away, its problem dating back to its first failure.
        let changes = engine.observe(db, Observation::Fine, 200);
        assert_eq!(ids(&changes), ["db", "api"]);
        assert_eq!(changes.into_iter().nth(1).map(|(_, c)| c), fired(1, "down"));
        assert_eq!(engine.observe_one(api, problem(), 201), None);
    }

    #[test]
    fn a_released_child_still_waits_for_its_own_for() {
        let db = alert("db", 0, None, &[]);
        let api = alert("api", 60, None, &["db"]);
        let mut engine = Engine::new();
        let problem = || Observation::Problem("down".to_owned());
        assert!(engine.observe_one(db, problem(), 0).is_some());
        assert_eq!(engine.observe_one(api, problem(), 10), None);
        // Only 20s of the child's 60s have passed.
        assert_eq!(ids(&engine.observe(db, Observation::Fine, 30)), ["db"]);
        assert_eq!(engine.observe_one(api, problem(), 70), fired(10, "down"));
    }

    #[test]
    fn an_urgent_child_is_released_at_once() {
        let db = alert("db", 0, None, &[]);
        let api = alert("api", 3600, None, &["db"]);
        let mut engine = Engine::new();
        assert!(
            engine
                .observe_one(db, Observation::Problem("down".to_owned()), 0)
                .is_some()
        );
        let urgent = Observation::Urgent("failed".to_owned());
        assert_eq!(engine.observe_one(api, urgent, 10), None);
        assert_eq!(
            ids(&engine.observe(db, Observation::Fine, 20)),
            ["db", "api"]
        );
    }

    #[test]
    fn a_chain_releases_one_level_at_a_time() {
        // host <- db <- api: `parents` holds all ancestors, as config
        // resolution leaves it.
        let host = alert("host", 0, None, &[]);
        let db = alert("db", 0, None, &["host"]);
        let api = alert("api", 0, None, &["db", "host"]);
        let mut engine = Engine::new();
        let problem = || Observation::Problem("down".to_owned());
        assert!(engine.observe_one(host, problem(), 0).is_some());
        assert_eq!(engine.observe_one(db, problem(), 1), None);
        assert_eq!(engine.observe_one(api, problem(), 2), None);

        // db is still down, so it fires and keeps holding api back.
        assert_eq!(
            ids(&engine.observe(host, Observation::Fine, 10)),
            ["host", "db"]
        );
        assert_eq!(engine.observe_one(api, problem(), 11), None);
        assert_eq!(
            ids(&engine.observe(db, Observation::Fine, 20)),
            ["db", "api"]
        );
    }

    #[test]
    fn a_grandchild_is_released_when_the_middle_recovered_first() {
        let host = alert("host", 0, None, &[]);
        let db = alert("db", 0, None, &["host"]);
        let api = alert("api", 0, None, &["db", "host"]);
        let mut engine = Engine::new();
        let problem = || Observation::Problem("down".to_owned());
        assert!(engine.observe_one(host, problem(), 0).is_some());
        assert_eq!(engine.observe_one(db, problem(), 1), None);
        assert_eq!(engine.observe_one(api, problem(), 2), None);
        // db recovers silently while host still holds api back.
        assert!(engine.observe(db, Observation::Fine, 5).is_empty());
        assert_eq!(
            ids(&engine.observe(host, Observation::Fine, 10)),
            ["host", "api"]
        );
    }

    #[test]
    fn a_warning_fires_as_a_warning_and_again_when_it_gets_worse() {
        let command = alert_with("command", 0, None, &[], Severity::Critical);
        let mut engine = Engine::new();
        let warning = || Observation::Warning("exit 1".to_owned());
        let failure = || Observation::Problem("exit 2".to_owned());
        assert_eq!(
            engine.observe_one(command, warning(), 0),
            fired_as(0, "exit 1", Severity::Warning)
        );
        assert_eq!(engine.observe_one(command, warning(), 10), None);
        assert_eq!(
            engine.observe_one(command, failure(), 20),
            fired_as(0, "exit 2", Severity::Critical)
        );
        // Back to a warning and worse again: already announced as critical.
        assert_eq!(engine.observe_one(command, warning(), 30), None);
        assert_eq!(engine.observe_one(command, failure(), 40), None);
        assert_eq!(
            engine.observe_one(command, Observation::Fine, 50),
            Some(Change::Resolved {
                since: 0,
                detail: "exit 2".to_owned(),
                severity: Severity::Critical,
            })
        );
    }

    #[test]
    fn a_warning_is_never_louder_than_the_alert() {
        let command = alert_with("command", 0, None, &[], Severity::Info);
        let mut engine = Engine::new();
        assert_eq!(
            engine.observe_one(command, Observation::Warning("exit 1".to_owned()), 0),
            fired_as(0, "exit 1", Severity::Info)
        );
    }
}
