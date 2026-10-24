use std::collections::VecDeque;

use crate::model::Timestamp;

/// Counts restarts within a sliding window, from a counter that the service
/// manager keeps.
#[derive(Debug, Default)]
pub(super) struct RestartWindow {
    /// The counter at the last check, `None` before the first.
    last: Option<u64>,
    /// When restarts were seen, oldest first, with how many each check saw.
    /// One entry per check at most, so `within / every` entries at most.
    seen: VecDeque<(Timestamp, u64)>,
}

impl RestartWindow {
    /// Takes the counter's value at `at` and returns the restarts seen within
    /// the last `within` seconds. The first value is only a baseline. A value
    /// lower than the last means the counter was reset, as systemd does on a
    /// manual start, which isn't a restart either.
    pub fn update(&mut self, counter: u64, at: Timestamp, within: Timestamp) -> u64 {
        if let Some(last) = self.last
            && counter > last
        {
            self.seen.push_back((at, counter - last));
        }
        self.last = Some(counter);
        while self
            .seen
            .front()
            .is_some_and(|&(seen_at, _)| at.saturating_sub(seen_at) >= within)
        {
            self.seen.pop_front();
        }
        self.seen.iter().map(|&(_, count)| count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: Timestamp = 3600;

    #[test]
    fn counts_within_the_window() {
        let mut window = RestartWindow::default();
        assert_eq!(window.update(7, 0, HOUR), 0, "a baseline, not restarts");
        assert_eq!(window.update(8, 600, HOUR), 1);
        assert_eq!(window.update(8, 900, HOUR), 1);
        // Two restarts between checks both count.
        assert_eq!(window.update(10, 1500, HOUR), 3);
        // The one at 600 slides out.
        assert_eq!(window.update(10, 4200, HOUR), 2);
        assert_eq!(window.update(10, 5100, HOUR), 0);
    }

    #[test]
    fn a_reset_counter_starts_over() {
        let mut window = RestartWindow::default();
        window.update(5, 0, HOUR);
        assert_eq!(window.update(6, 10, HOUR), 1);
        assert_eq!(window.update(0, 20, HOUR), 1, "a reset isn't a restart");
        assert_eq!(window.update(1, 30, HOUR), 2);
    }
}
