//! Reads `/proc/meminfo`. The parser is pure so it's tested on any OS.

use super::Usage;

#[cfg(target_os = "linux")]
pub fn read() -> std::io::Result<Usage> {
    let text = std::fs::read_to_string("/proc/meminfo")?;
    parse(&text).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "/proc/meminfo has no MemTotal/MemAvailable",
        )
    })
}

/// `MemAvailable` (Linux 3.14+) is the kernel's estimate of memory that can be
/// given to programs without swapping: free memory plus page cache and other
/// reclaimable memory. Used is everything else, as `free` and `htop` show it.
fn parse(text: &str) -> Option<Usage> {
    let mut total = None;
    let mut available = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let slot = match key {
            "MemTotal" => &mut total,
            "MemAvailable" => &mut available,
            _ => continue,
        };
        let kib: u64 = value.trim().strip_suffix(" kB")?.parse().ok()?;
        *slot = Some(kib * 1024);
    }
    let (total, available) = (total?, available?);
    Some(Usage {
        used: total.checked_sub(available)?,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo() {
        let text = "MemTotal:       16000000 kB\n\
                    MemFree:         1000000 kB\n\
                    MemAvailable:   12000000 kB\n\
                    Buffers:          500000 kB\n\
                    Cached:          9000000 kB\n";
        assert_eq!(
            parse(text),
            Some(Usage {
                used: 4_000_000 * 1024,
                total: 16_000_000 * 1024,
            })
        );
    }

    #[test]
    fn rejects_incomplete_or_garbage() {
        assert_eq!(parse("MemTotal: 16000000 kB\n"), None);
        assert_eq!(parse("MemTotal: x kB\nMemAvailable: 1 kB\n"), None);
        assert_eq!(parse("MemTotal: 1 kB\nMemAvailable: 2 kB\n"), None);
    }
}
