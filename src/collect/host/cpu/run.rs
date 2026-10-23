use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, sleep};

use crate::config::Metric;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

#[cfg(target_os = "linux")]
use super::linux::read;
#[cfg(target_os = "macos")]
use super::macos::read;
use super::ticks::usage;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("CPU collector supports only Linux and macOS");

/// The first value is averaged over a shorter window so it shows up right
/// after start instead of one full `every` later.
const FIRST_WINDOW: Duration = Duration::from_secs(1);

pub async fn run(config: &'static Metric) {
    let mut prev = read().ok();
    sleep(FIRST_WINDOW.min(config.every)).await;

    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
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
