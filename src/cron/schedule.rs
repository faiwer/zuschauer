use std::fmt;

use crate::model::Timestamp;

use super::civil::Civil;
use super::field::Field;

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const WEEKDAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// Long enough to reach any 29 February, which can be 8 years apart.
const SEARCH_YEARS: i32 = 9;

/// A cron expression: `minute hour day-of-month month day-of-week`, or one
/// of `@hourly`, `@daily`, `@weekly`, `@monthly`, `@yearly`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    /// As written, for display.
    text: String,
    minute: Field,
    hour: Field,
    day: Field,
    month: Field,
    weekday: Field,
}

impl Schedule {
    pub fn parse(text: &str) -> Result<Self, String> {
        let expanded = match text {
            "@hourly" => "0 * * * *",
            "@daily" | "@midnight" => "0 0 * * *",
            "@weekly" => "0 0 * * 0",
            "@monthly" => "0 0 1 * *",
            "@yearly" | "@annually" => "0 0 1 1 *",
            _ if text.starts_with('@') => {
                return Err(format!("\"{text}\" is not a known shortcut"));
            }
            _ => text,
        };
        let fields: Vec<&str> = expanded.split_whitespace().collect();
        let [minute, hour, day, month, weekday] = fields[..] else {
            return Err(format!(
                "\"{text}\" has {} fields, a cron expression has 5: minute hour day month weekday",
                fields.len()
            ));
        };
        let field = |name: &str, text: &str, min, max, names: &[&str]| {
            Field::parse(text, min, max, names).map_err(|e| format!("{name} \"{text}\": {e}"))
        };
        Ok(Self {
            text: text.to_owned(),
            minute: field("minute", minute, 0, 59, &[])?,
            hour: field("hour", hour, 0, 23, &[])?,
            day: field("day", day, 1, 31, &[])?,
            month: field("month", month, 1, 12, &MONTHS)?,
            weekday: field("weekday", weekday, 0, 7, &WEEKDAYS)?.fold_seven_into_zero(),
        })
    }

    /// The first due time after `at`, or `None` if it never comes, like
    /// `0 0 31 2 *`.
    pub fn next_after(&self, at: Timestamp) -> Option<Timestamp> {
        let mut from = Civil::local(at).next_minute();
        // Around a daylight-saving jump back, the next wall-clock match can
        // still be earlier than `at`.
        loop {
            let due = self.next_civil(from)?;
            let timestamp = due.timestamp()?;
            if timestamp > at {
                return Some(timestamp);
            }
            from = due.next_minute();
        }
    }

    /// The first matching minute at or after `from`.
    fn next_civil(&self, from: Civil) -> Option<Civil> {
        let mut c = from;
        while c.year - from.year <= SEARCH_YEARS {
            if !self.month.matches(c.month) {
                c = c.next_month();
            } else if !self.day_matches(c) {
                c = c.next_day();
            } else if !self.hour.matches(c.hour) {
                c = c.next_hour();
            } else if !self.minute.matches(c.minute) {
                c = c.next_minute();
            } else {
                return Some(c);
            }
        }
        None
    }

    /// When both day fields are restricted, cron runs on either. `0 0 1 * 1`
    /// is the 1st of the month and every Monday.
    fn day_matches(&self, c: Civil) -> bool {
        let day = self.day.matches(c.day);
        let weekday = self.weekday.matches(c.weekday());
        if self.day.star || self.weekday.star {
            day && weekday
        } else {
            day || weekday
        }
    }
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
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

    fn next(text: &str, from: Civil) -> Option<Civil> {
        Schedule::parse(text).unwrap().next_civil(from)
    }

    // 2026-10-23 is a Friday.
    const FRIDAY_NOON: Civil = Civil {
        year: 2026,
        month: 10,
        day: 23,
        hour: 12,
        minute: 0,
    };

    #[test]
    fn finds_the_next_due_time() {
        assert_eq!(
            next("0 3 * * *", FRIDAY_NOON),
            Some(civil(2026, 10, 24, 3, 0))
        );
        assert_eq!(next("@daily", FRIDAY_NOON), Some(civil(2026, 10, 24, 0, 0)));
        assert_eq!(next("*/15 * * * *", FRIDAY_NOON), Some(FRIDAY_NOON));
        assert_eq!(
            next("30 2 * * 1-5", FRIDAY_NOON),
            Some(civil(2026, 10, 26, 2, 30))
        );
        assert_eq!(
            next("0 9,18 * * *", FRIDAY_NOON),
            Some(civil(2026, 10, 23, 18, 0))
        );
        assert_eq!(
            next("0 0 1 * *", FRIDAY_NOON),
            Some(civil(2026, 11, 1, 0, 0))
        );
        assert_eq!(
            next("0 0 29 2 *", FRIDAY_NOON),
            Some(civil(2028, 2, 29, 0, 0))
        );
        assert_eq!(next("0 0 31 2 *", FRIDAY_NOON), None);
    }

    #[test]
    fn runs_on_either_restricted_day_field() {
        // The 1st, or a Monday: Monday the 26th comes first.
        assert_eq!(
            next("0 0 1 * mon", FRIDAY_NOON),
            Some(civil(2026, 10, 26, 0, 0))
        );
        // `*/2` counts as unrestricted, so both must match: an odd day that's
        // a Sunday.
        assert_eq!(
            next("0 0 */2 * 0", FRIDAY_NOON),
            Some(civil(2026, 10, 25, 0, 0))
        );
        assert_eq!(
            next("0 0 */2 * 0", civil(2026, 10, 26, 0, 0)),
            Some(civil(2026, 11, 1, 0, 0))
        );
    }

    #[test]
    fn rejects_bad_expressions() {
        for text in [
            "",
            "* * * *",
            "* * * * * *",
            "61 * * * *",
            "* * 0 * *",
            "@reboot",
        ] {
            assert!(Schedule::parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn next_after_is_a_later_whole_minute() {
        let schedule = Schedule::parse("* * * * *").unwrap();
        let at = 1_790_000_030;
        let due = schedule.next_after(at).unwrap();
        assert!(due > at && due - at <= 60, "{at} {due}");
        assert_eq!(schedule.next_after(due).unwrap() - due, 60);
    }
}
