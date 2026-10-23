//! Used space of the filesystem holding each configured path. Reads the
//! filesystem's own counters, like `df`, so it's instant regardless of how
//! many files there are.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux::read;
#[cfg(target_os = "macos")]
use macos::read;

use std::io;

use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};

use crate::config::Mount;
use crate::model::{EntityId, MetricKind, Sample};
use crate::store::store;

/// Filesystem block counters, as reported by the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Blocks {
    block_size: u64,
    total: u64,
    /// Free, including blocks reserved for root.
    free: u64,
    /// Free for non-root users.
    available: u64,
}

impl Blocks {
    /// `df`'s formula: 100% means non-root programs can't write anymore, even
    /// if root-reserved blocks are still free.
    fn percent(self) -> Option<f32> {
        let used = self.total.checked_sub(self.free)?;
        let usable = used + self.available;
        if usable == 0 {
            return None;
        }
        Some(used as f32 / usable as f32 * 100.0)
    }
}

pub async fn run(mount: &'static Mount) {
    let entity = EntityId::Mount(mount.path.clone());
    let mut ticker = interval(mount.metric.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        // A hung network mount can block the call forever, so it runs on the
        // blocking pool instead of the runtime's only thread.
        let result = spawn_blocking(|| read(&mount.path))
            .await
            .unwrap_or_else(|e| Err(io::Error::other(e)));
        let blocks = match result {
            Ok(blocks) => blocks,
            Err(e) => {
                eprintln!("storage {}: {e}", mount.path.display());
                continue;
            }
        };
        let Some(percent) = blocks.percent() else {
            eprintln!(
                "storage {}: implausible reading {blocks:?}",
                mount.path.display()
            );
            continue;
        };
        store().push(entity.clone(), MetricKind::Storage, Sample::now(percent));
        eprintln!(
            "storage {}: {percent:.1}% ({} GiB available)",
            mount.path.display(),
            blocks.available * blocks.block_size >> 30
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(total: u64, free: u64, available: u64) -> Blocks {
        Blocks {
            block_size: 4096,
            total,
            free,
            available,
        }
    }

    #[test]
    fn matches_df_with_root_reserve() {
        // 100 blocks, 80 used, 5 reserved for root: 80 / (80 + 15).
        let percent = blocks(100, 20, 15).percent().unwrap();
        assert!((percent - 84.21).abs() < 0.01, "{percent}");
    }

    #[test]
    fn handles_edge_cases() {
        assert_eq!(blocks(100, 100, 100).percent(), Some(0.0));
        assert_eq!(blocks(100, 0, 0).percent(), Some(100.0));
        assert_eq!(blocks(0, 0, 0).percent(), None);
        assert_eq!(blocks(100, 120, 120).percent(), None);
    }
}
