use crate::cron::Civil;
use crate::model::{Status, Timestamp, now};

pub fn status_label(status: Option<Status>) -> String {
    match status {
        None => "pending".to_owned(),
        Some(Status::Running) => "running".to_owned(),
        Some(Status::Paused) => "paused".to_owned(),
        Some(Status::Gone { since }) => format!("gone {}", ago(since)),
        Some(Status::Up { since }) => format!("up {}", ago(since)),
        Some(Status::Down { since }) => format!("down {}", ago(since)),
    }
}

pub fn mib(bytes: f32) -> String {
    format!("{:.0} MiB", bytes / (1024.0 * 1024.0))
}

/// Compact age: `42s`, `12m`, `5h`, `3d`.
pub fn ago(at: Timestamp) -> String {
    span(now().saturating_sub(at))
}

/// Like `ago`, for the future.
pub fn until(at: Timestamp) -> String {
    span(at.saturating_sub(now()))
}

/// Local time, like `Apr 28, 2025, 15:24`.
pub fn date_time(at: Timestamp) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let c = Civil::local(at);
    format!(
        "{} {}, {}, {:02}:{:02}",
        MONTHS[c.month as usize - 1],
        c.day,
        c.year,
        c.hour,
        c.minute
    )
}

pub fn span(seconds: Timestamp) -> String {
    match seconds {
        0..120 => format!("{seconds}s"),
        120..7200 => format!("{}m", seconds / 60),
        7200..172_800 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}
