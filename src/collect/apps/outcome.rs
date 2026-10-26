use crate::model::{EntityId, Status, Timestamp, now};
use crate::store::store;

/// What one attempt came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    Up,
    Warn,
    Down,
}

/// Stores one attempt's status and, on failure, the reason. Returns the new
/// status, to be passed back as `prev` next time. What a success measured is
/// the caller's to store.
pub(super) fn record(
    entity: &EntityId,
    prev: Option<Status>,
    outcome: Result<(), String>,
) -> Status {
    let verdict = match outcome {
        Ok(()) => Verdict::Up,
        Err(_) => Verdict::Down,
    };
    record_verdict(entity, prev, verdict, outcome.err())
}

/// Like `record`, for an attempt that can also end in a warning. `error`:
/// the reason, unless it's `Up`.
pub(super) fn record_verdict(
    entity: &EntityId,
    prev: Option<Status>,
    verdict: Verdict,
    error: Option<String>,
) -> Status {
    let status = streak(prev, verdict, now());
    let mut store = store();
    store.set_status(entity.clone(), status);
    store.set_error(entity.clone(), error);
    status
}

/// Keeps `since` while the verdict stays the same.
fn streak(prev: Option<Status>, verdict: Verdict, at: Timestamp) -> Status {
    match (prev, verdict) {
        (Some(Status::Up { since }), Verdict::Up) => Status::Up { since },
        (Some(Status::Warn { since }), Verdict::Warn) => Status::Warn { since },
        (Some(Status::Down { since }), Verdict::Down) => Status::Down { since },
        (_, Verdict::Up) => Status::Up { since: at },
        (_, Verdict::Warn) => Status::Warn { since: at },
        (_, Verdict::Down) => Status::Down { since: at },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_since_within_a_streak() {
        let up = streak(None, Verdict::Up, 10);
        assert_eq!(up, Status::Up { since: 10 });
        assert_eq!(streak(Some(up), Verdict::Up, 20), Status::Up { since: 10 });
        let down = streak(Some(up), Verdict::Down, 30);
        assert_eq!(down, Status::Down { since: 30 });
        assert_eq!(
            streak(Some(down), Verdict::Down, 40),
            Status::Down { since: 30 }
        );
        let warn = streak(Some(down), Verdict::Warn, 50);
        assert_eq!(warn, Status::Warn { since: 50 });
        assert_eq!(
            streak(Some(warn), Verdict::Warn, 60),
            Status::Warn { since: 50 }
        );
        assert_eq!(
            streak(Some(warn), Verdict::Up, 70),
            Status::Up { since: 70 }
        );
    }
}
