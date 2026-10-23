/// Cumulative CPU time since boot, in clock ticks, summed over all cores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Ticks {
    pub idle: u64,
    pub total: u64,
}

/// Busy share between two readings, in percent. `None` when there's nothing
/// meaningful to report: no time passed, or a counter went backwards (a CPU
/// went offline, or a 32-bit counter wrapped on macOS).
pub(super) fn usage(prev: Ticks, cur: Ticks) -> Option<f32> {
    let total = cur.total.checked_sub(prev.total)?;
    let idle = cur.idle.checked_sub(prev.idle)?;
    if total == 0 || idle > total {
        return None;
    }
    Some((total - idle) as f32 / total as f32 * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(idle: u64, total: u64) -> Ticks {
        Ticks { idle, total }
    }

    #[test]
    fn computes_busy_share() {
        assert_eq!(usage(ticks(100, 200), ticks(175, 300)), Some(25.0));
        assert_eq!(usage(ticks(0, 0), ticks(0, 100)), Some(100.0));
        assert_eq!(usage(ticks(0, 0), ticks(100, 100)), Some(0.0));
    }

    #[test]
    fn skips_unusable_intervals() {
        assert_eq!(usage(ticks(10, 100), ticks(10, 100)), None);
        assert_eq!(usage(ticks(10, 100), ticks(5, 90)), None);
        assert_eq!(usage(ticks(10, 100), ticks(80, 150)), None);
    }
}
