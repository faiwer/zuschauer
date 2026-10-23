use std::time::Instant;

/// CPU time used between two readings as a share of all `cores` over the
/// wall time between them. `None` when the counter went backwards, which
/// means the container restarted in between.
pub(super) fn cpu_percent(
    prev_usec: u64,
    prev_at: Instant,
    cur_usec: u64,
    cur_at: Instant,
    cores: u32,
) -> Option<f32> {
    let used = cur_usec.checked_sub(prev_usec)?;
    let wall = cur_at.checked_duration_since(prev_at)?.as_micros() as f64;
    if wall == 0.0 {
        return None;
    }
    let percent = used as f64 / (wall * f64::from(cores.max(1))) * 100.0;
    Some(percent.min(100.0) as f32)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn cpu_share_of_all_cores() {
        let start = Instant::now();
        let later = start + Duration::from_secs(10);
        // 10 s of wall time on 4 cores = 40 s of CPU time available.
        assert_eq!(cpu_percent(0, start, 10_000_000, later, 4), Some(25.0));
        assert_eq!(cpu_percent(0, start, 40_000_000, later, 4), Some(100.0));
    }

    #[test]
    fn cpu_skips_restarts_and_zero_windows() {
        let start = Instant::now();
        let later = start + Duration::from_secs(10);
        assert_eq!(cpu_percent(5_000_000, start, 1_000_000, later, 4), None);
        assert_eq!(cpu_percent(0, start, 1_000_000, start, 4), None);
    }
}
