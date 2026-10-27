use crate::cron::Civil;
use crate::model::{Status, Timestamp, now};

/// The state, with how long for: `up for 3d`.
pub fn status_for(status: Option<Status>) -> String {
    match status {
        None => "no result yet".to_owned(),
        Some(Status::Running) => "running".to_owned(),
        Some(Status::Paused) => "paused".to_owned(),
        Some(Status::Gone { since }) => format!("gone for {}", ago(since)),
        Some(Status::Up { since }) => format!("up for {}", ago(since)),
        Some(Status::Warn { since }) => format!("warning for {}", ago(since)),
        Some(Status::Down { since }) => format!("down for {}", ago(since)),
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

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Local time: `today 15:24`, `yesterday 15:24`, else `Apr 28, 2025, 15:24`.
pub fn date_time(at: Timestamp) -> String {
    let c = Civil::local(at);
    let day = |c: Civil| (c.year, c.month, c.day);
    let today = Civil::local(now());
    // A day before today's midnight, not 24h ago: days around a
    // daylight-saving jump last 23 or 25 hours.
    let yesterday = Civil {
        hour: 0,
        minute: 0,
        ..today
    }
    .timestamp()
    .map(|midnight| Civil::local(midnight.saturating_sub(1)));
    if day(c) == day(today) {
        return format!("today {:02}:{:02}", c.hour, c.minute);
    }
    if yesterday.is_some_and(|yesterday| day(c) == day(yesterday)) {
        return format!("yesterday {:02}:{:02}", c.hour, c.minute);
    }
    format!(
        "{} {}, {}, {:02}:{:02}",
        MONTHS[c.month as usize - 1],
        c.day,
        c.year,
        c.hour,
        c.minute
    )
}

/// Local time of day, like `15:24`.
pub fn clock(at: Timestamp) -> String {
    let c = Civil::local(at);
    format!("{:02}:{:02}", c.hour, c.minute)
}

/// Like `date_time`, without the time unless it's today: `Apr 28, 2025`.
pub fn date(at: Timestamp) -> String {
    let c = Civil::local(at);
    let today = Civil::local(now());
    if (c.year, c.month, c.day) == (today.year, today.month, today.day) {
        return date_time(at);
    }
    format!("{} {}, {}", MONTHS[c.month as usize - 1], c.day, c.year)
}

pub fn span(seconds: Timestamp) -> String {
    match seconds {
        0..120 => format!("{seconds}s"),
        120..7200 => format!("{}m", seconds / 60),
        7200..172_800 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_today_and_yesterday() {
        let at = now();
        assert!(date_time(at).starts_with("today "), "{}", date_time(at));
        let today = Civil::local(at);
        let midnight = Civil {
            hour: 0,
            minute: 0,
            ..today
        }
        .timestamp()
        .unwrap();
        let before = date_time(midnight - 1);
        assert!(before.starts_with("yesterday "), "{before}");
        let older = date_time(midnight - 2 * 86_400);
        assert!(older.contains(", "), "{older}");
    }
}
