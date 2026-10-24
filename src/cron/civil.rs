use crate::model::Timestamp;

/// A local date and time to the minute, as a wall clock shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Civil {
    pub year: i32,
    /// 1..=12
    pub month: u32,
    /// 1..=31
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Civil {
    /// 0 is Sunday.
    pub fn weekday(self) -> u32 {
        // Sakamoto's method.
        const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let year = if self.month < 3 {
            self.year - 1
        } else {
            self.year
        };
        let day = year + year / 4 - year / 100
            + year / 400
            + OFFSETS[self.month as usize - 1]
            + self.day as i32;
        day.rem_euclid(7) as u32
    }

    pub fn next_minute(self) -> Self {
        if self.minute < 59 {
            Self {
                minute: self.minute + 1,
                ..self
            }
        } else {
            self.next_hour()
        }
    }

    pub fn next_hour(self) -> Self {
        if self.hour < 23 {
            Self {
                hour: self.hour + 1,
                minute: 0,
                ..self
            }
        } else {
            self.next_day()
        }
    }

    pub fn next_day(self) -> Self {
        if self.day < days_in_month(self.year, self.month) {
            Self {
                day: self.day + 1,
                hour: 0,
                minute: 0,
                ..self
            }
        } else {
            self.next_month()
        }
    }

    pub fn next_month(self) -> Self {
        let (year, month) = if self.month < 12 {
            (self.year, self.month + 1)
        } else {
            (self.year + 1, 1)
        };
        Self {
            year,
            month,
            day: 1,
            hour: 0,
            minute: 0,
        }
    }

    /// The local wall-clock time at `at`, without seconds.
    pub fn local(at: Timestamp) -> Self {
        let time = at.into();
        // SAFETY: an all-zero `tm` is valid, and both pointers outlive the call.
        let tm = unsafe {
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&time, &mut tm);
            tm
        };
        Self {
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
        }
    }

    /// When the local wall clock shows this time. A time skipped by a
    /// daylight-saving jump comes out just after the jump; a time shown twice
    /// comes out as one of the two.
    pub fn timestamp(self) -> Option<Timestamp> {
        // SAFETY: an all-zero `tm` is valid, and the pointer outlives the call.
        let time = unsafe {
            let mut tm: libc::tm = std::mem::zeroed();
            tm.tm_year = self.year - 1900;
            tm.tm_mon = self.month as i32 - 1;
            tm.tm_mday = self.day as i32;
            tm.tm_hour = self.hour as i32;
            tm.tm_min = self.minute as i32;
            tm.tm_isdst = -1;
            libc::mktime(&mut tm)
        };
        Timestamp::try_from(time).ok()
    }
}

/// Like `2026-10-25 03:00`, in local time.
pub fn format_local(at: Timestamp) -> String {
    let c = Civil::local(at);
    format!(
        "{}-{:02}-{:02} {:02}:{:02}",
        c.year, c.month, c.day, c.hour, c.minute
    )
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn civil(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Civil {
        Civil {
            year,
            month,
            day,
            hour,
            minute,
        }
    }

    #[test]
    fn knows_weekdays() {
        assert_eq!(civil(2026, 10, 24, 0, 0).weekday(), 6, "a Saturday");
        assert_eq!(civil(2024, 2, 29, 0, 0).weekday(), 4, "a Thursday");
        assert_eq!(civil(2000, 1, 1, 0, 0).weekday(), 6, "a Saturday");
    }

    #[test]
    fn steps_over_month_and_year_ends() {
        assert_eq!(
            civil(2026, 12, 31, 23, 59).next_minute(),
            civil(2027, 1, 1, 0, 0)
        );
        assert_eq!(
            civil(2024, 2, 28, 5, 5).next_day(),
            civil(2024, 2, 29, 0, 0)
        );
        assert_eq!(civil(2026, 2, 28, 5, 5).next_day(), civil(2026, 3, 1, 0, 0));
        assert_eq!(
            civil(2026, 4, 30, 23, 0).next_hour(),
            civil(2026, 5, 1, 0, 0)
        );
    }

    #[test]
    fn converts_to_timestamps_and_back() {
        let at = 1_790_000_000;
        let c = Civil::local(at);
        let back = c.timestamp().unwrap();
        // Seconds are dropped.
        assert!(at - back < 60, "{at} {back}");
        assert_eq!(Civil::local(back), c);
    }
}
