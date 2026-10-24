/// The values one field of a cron expression matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Field {
    /// Bit `n` set: matches `n`.
    bits: u64,
    /// Written starting with `*`. Cron combines the day fields differently
    /// when one of them is.
    pub star: bool,
}

impl Field {
    /// `names[i]` stands for the value `min + i`. Each comma-separated item
    /// is `*`, `n` or `a-b`, optionally followed by `/step`; `n/step` means
    /// `n-max/step`.
    pub fn parse(text: &str, min: u32, max: u32, names: &[&str]) -> Result<Self, String> {
        let mut bits = 0;
        for item in text.split(',') {
            let (range, step) = match item.split_once('/') {
                Some((range, step)) => {
                    let step: u32 = step
                        .parse()
                        .ok()
                        .filter(|&step| step > 0)
                        .ok_or_else(|| format!("\"{step}\" is not a step"))?;
                    (range, Some(step))
                }
                None => (item, None),
            };
            let (from, to) = match range.split_once('-') {
                _ if range == "*" => (min, max),
                Some((from, to)) => (value(from, min, max, names)?, value(to, min, max, names)?),
                None => {
                    let from = value(range, min, max, names)?;
                    (from, if step.is_some() { max } else { from })
                }
            };
            if from > to {
                return Err(format!("\"{range}\" is a backwards range"));
            }
            for v in (from..=to).step_by(step.unwrap_or(1) as usize) {
                bits |= 1 << v;
            }
        }
        Ok(Self {
            bits,
            star: text.starts_with('*'),
        })
    }

    pub fn matches(&self, value: u32) -> bool {
        self.bits >> value & 1 == 1
    }

    /// For day-of-week, where both 0 and 7 are Sunday.
    pub fn fold_seven_into_zero(mut self) -> Self {
        if self.matches(7) {
            self.bits = (self.bits | 1) & !(1 << 7);
        }
        self
    }
}

fn value(text: &str, min: u32, max: u32, names: &[&str]) -> Result<u32, String> {
    let value = match names
        .iter()
        .position(|name| name.eq_ignore_ascii_case(text))
    {
        Some(i) => min + i as u32,
        None => text
            .parse()
            .map_err(|_| format!("\"{text}\" is not a number"))?,
    };
    if !(min..=max).contains(&value) {
        return Err(format!("{value} is outside {min}-{max}"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(field: Field, max: u32) -> Vec<u32> {
        (0..=max).filter(|&v| field.matches(v)).collect()
    }

    #[test]
    fn parses_items() {
        let minute = |text| Field::parse(text, 0, 59, &[]).unwrap();
        assert_eq!(values(minute("5"), 59), [5]);
        assert_eq!(values(minute("1,15,30"), 59), [1, 15, 30]);
        assert_eq!(values(minute("10-13"), 59), [10, 11, 12, 13]);
        assert_eq!(values(minute("*/15"), 59), [0, 15, 30, 45]);
        assert_eq!(values(minute("50/5"), 59), [50, 55]);
        assert_eq!(values(minute("0-10/5,59"), 59), [0, 5, 10, 59]);
        assert!(minute("*").star && minute("*/15").star && !minute("5").star);
    }

    #[test]
    fn parses_names_and_sunday_as_seven() {
        let days = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
        let weekday = |text| {
            Field::parse(text, 0, 7, &days)
                .unwrap()
                .fold_seven_into_zero()
        };
        assert_eq!(values(weekday("Mon-Fri"), 7), [1, 2, 3, 4, 5]);
        assert_eq!(values(weekday("7"), 7), [0]);
        assert_eq!(values(weekday("fri-7"), 7), [0, 5, 6]);
    }

    #[test]
    fn rejects_bad_items() {
        for text in ["60", "a", "5-1", "*/0", "1-", "", "*/x"] {
            assert!(Field::parse(text, 0, 59, &[]).is_err(), "{text}");
        }
    }
}
