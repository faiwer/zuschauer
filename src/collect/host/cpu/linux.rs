//! Reads `/proc/stat`. The parser is pure so it's tested on any OS.

use super::ticks::Ticks;

#[cfg(target_os = "linux")]
pub fn read() -> std::io::Result<Ticks> {
    let text = std::fs::read_to_string("/proc/stat")?;
    parse(&text).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unexpected /proc/stat format",
        )
    })
}

/// The first line is the sum over all cores:
/// `cpu user nice system idle iowait irq softirq steal guest guest_nice`.
/// `guest` and `guest_nice` are already included in `user` and `nice`, so only
/// the first 8 fields make up the total.
fn parse(text: &str) -> Option<Ticks> {
    let line = text.lines().next()?.strip_prefix("cpu ")?;
    let fields = line
        .split_whitespace()
        .take(8)
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()
        .ok()?;
    if fields.len() < 4 {
        return None;
    }
    let iowait = fields.get(4).copied().unwrap_or(0);
    Some(Ticks {
        idle: fields[3] + iowait,
        total: fields.iter().sum(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aggregate_line() {
        let text = "cpu  100 5 50 800 20 3 2 10 7 0\ncpu0 50 2 25 400 10 1 1 5 3 0\n";
        assert_eq!(
            parse(text),
            Some(Ticks {
                idle: 820,
                total: 990
            })
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("intr 1 2 3\n"), None);
        assert_eq!(parse("cpu  1 2\n"), None);
        assert_eq!(parse("cpu  1 x 3 4\n"), None);
    }
}
