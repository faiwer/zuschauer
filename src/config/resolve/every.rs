use std::time::Duration;

use crate::config::types::Dur;

pub(super) const DEFAULT_EVERY: Duration = Duration::from_secs(60);
const MIN_EVERY: Duration = Duration::from_secs(1);

pub(super) fn every(
    at: &str,
    raw: Option<Dur>,
    default: Duration,
    errors: &mut Vec<String>,
) -> Duration {
    let every = raw.map_or(default, Dur::get);
    if every < MIN_EVERY {
        errors.push(format!(
            "{at}.every: must be at least {}",
            Dur::from(MIN_EVERY)
        ));
    }
    every
}
