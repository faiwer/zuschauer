use crate::model::{EntityId, Status, Timestamp, now};
use crate::store::store;

/// Stores one attempt's status and, on failure, the reason. Returns the new
/// status, to be passed back as `prev` next time. What a success measured is
/// the caller's to store.
pub(super) fn record(
    entity: &EntityId,
    prev: Option<Status>,
    outcome: Result<(), String>,
) -> Status {
    let status = streak(prev, outcome.is_ok(), now());
    let mut store = store();
    store.set_status(entity.clone(), status);
    store.set_error(entity.clone(), outcome.err());
    status
}

/// Keeps `since` while the outcome stays the same.
fn streak(prev: Option<Status>, up: bool, at: Timestamp) -> Status {
    match (prev, up) {
        (Some(Status::Up { since }), true) => Status::Up { since },
        (Some(Status::Down { since }), false) => Status::Down { since },
        (_, true) => Status::Up { since: at },
        (_, false) => Status::Down { since: at },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_since_within_a_streak() {
        let up = streak(None, true, 10);
        assert_eq!(up, Status::Up { since: 10 });
        assert_eq!(streak(Some(up), true, 20), Status::Up { since: 10 });
        let down = streak(Some(up), false, 30);
        assert_eq!(down, Status::Down { since: 30 });
        assert_eq!(streak(Some(down), false, 40), Status::Down { since: 30 });
        assert_eq!(streak(Some(down), true, 50), Status::Up { since: 50 });
    }
}
