//! Total CPU usage across all cores, as the average over each `every`
//! interval.

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("CPU collector supports only Linux and macOS");

#[cfg(target_os = "linux")]
use linux::read;
#[cfg(target_os = "macos")]
use macos::read;

use tokio::time::{MissedTickBehavior, interval};

use crate::config::Metric;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

/// Cumulative CPU time since boot, in clock ticks, summed over all cores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ticks {
    idle: u64,
    total: u64,
}

pub async fn run(config: &'static Metric) {
    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut prev = None;
    loop {
        ticker.tick().await;
        let cur = match read() {
            Ok(ticks) => ticks,
            Err(e) => {
                eprintln!("cpu: {e}");
                continue;
            }
        };
        if let Some(prev) = prev
            && let Some(percent) = usage(prev, cur)
        {
            store().push(EntityId::Host, MetricKind::Cpu, Sample::now(percent));
            eprintln!("cpu: {percent:.1}%");
        }
        prev = Some(cur);
    }
}

/// Busy share between two readings, in percent. `None` when there's nothing
/// meaningful to report: no time passed, or a counter went backwards (a CPU
/// went offline, or a 32-bit counter wrapped on macOS).
fn usage(prev: Ticks, cur: Ticks) -> Option<f32> {
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
