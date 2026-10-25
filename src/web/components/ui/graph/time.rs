use crate::cron::Civil;
use crate::model::Timestamp;

/// At most this many ticks along the time axis.
const MAX_TICKS: Timestamp = 6;
/// Round steps, in seconds.
const STEPS: [Timestamp; 15] = [
    10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 10_800, 21_600, 43_200, 86_400,
];
const DAY: Timestamp = 86_400;

/// Round local times within `first..=last`, and the step between them.
pub fn time_ticks(first: Timestamp, last: Timestamp) -> (Vec<Timestamp>, Timestamp) {
    let span = last - first;
    let step = STEPS
        .into_iter()
        .find(|&step| span / step < MAX_TICKS)
        .unwrap_or(DAY);
    let offset = utc_offset(first);
    let start = (first + offset).div_ceil(step) * step - offset;
    let ticks = (0..)
        .map(|i| start + i * step)
        .take_while(|&at| at <= last)
        .collect();
    (ticks, step)
}

/// How far local time is ahead of UTC, as seconds into the day.
fn utc_offset(at: Timestamp) -> Timestamp {
    let local = Civil::local(at);
    let local_seconds = local.hour * 3600 + local.minute * 60 + at % 60;
    (local_seconds + DAY - at % DAY) % DAY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_are_round_and_within_the_span() {
        let first = 1_800_000_000 + 30;
        let (ticks, step) = time_ticks(first, first + 150);
        assert_eq!(step, 30);
        assert!(ticks.iter().all(|&t| (first..=first + 150).contains(&t)));
        assert!(
            ticks
                .iter()
                .all(|&t| (t + utc_offset(t)).is_multiple_of(step))
        );
        assert_eq!(ticks.len(), 6);
    }

    #[test]
    fn longer_spans_take_longer_steps() {
        assert_eq!(time_ticks(0, 8 * 3600).1, 7200);
        assert_eq!(time_ticks(0, 120).1, 30);
    }
}
