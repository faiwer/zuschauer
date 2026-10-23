//! Real memory usage: what programs hold, excluding cache the kernel can
//! drop at any time.

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux::read;
#[cfg(target_os = "macos")]
use macos::read;

use tokio::time::{MissedTickBehavior, interval};

use crate::config::Metric;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

/// In bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Usage {
    used: u64,
    total: u64,
}

impl Usage {
    fn percent(self) -> Option<f32> {
        if self.total == 0 || self.used > self.total {
            return None;
        }
        Some(self.used as f32 / self.total as f32 * 100.0)
    }
}

pub async fn run(config: &'static Metric) {
    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let usage = match read() {
            Ok(usage) => usage,
            Err(e) => {
                eprintln!("memory: {e}");
                continue;
            }
        };
        let Some(percent) = usage.percent() else {
            eprintln!("memory: implausible reading {usage:?}");
            continue;
        };
        store().push(EntityId::Host, MetricKind::Memory, Sample::now(percent));
        eprintln!(
            "memory: {percent:.1}% ({} / {} MiB)",
            usage.used >> 20,
            usage.total >> 20
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_percent() {
        assert_eq!(Usage { used: 1, total: 4 }.percent(), Some(25.0));
        assert_eq!(Usage { used: 0, total: 0 }.percent(), None);
        assert_eq!(Usage { used: 5, total: 4 }.percent(), None);
    }
}
