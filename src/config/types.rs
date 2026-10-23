use std::fmt;
use std::time::Duration;

use serde::Deserialize;

/// A percentage written as `"85%"`. Always within `0..=100`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize)]
#[serde(try_from = "String")]
pub struct Percent(f32);

impl TryFrom<String> for Percent {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let number = s
            .strip_suffix('%')
            .ok_or_else(|| format!("expected a percentage like \"85%\", got \"{s}\""))?;
        let value: f32 = number
            .trim()
            .parse()
            .map_err(|_| format!("\"{s}\" is not a valid percentage"))?;
        if !(0.0..=100.0).contains(&value) {
            return Err(format!("\"{s}\" must be between 0% and 100%"));
        }
        Ok(Self(value))
    }
}

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

/// A human-readable duration such as `"60s"`, `"10m"` or `"14d"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct Dur(Duration);

impl Dur {
    pub fn get(self) -> Duration {
        self.0
    }
}

impl From<Duration> for Dur {
    fn from(d: Duration) -> Self {
        Self(d)
    }
}

impl TryFrom<String> for Dur {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        humantime::parse_duration(&s).map(Self).map_err(|e| {
            format!("\"{s}\" is not a valid duration ({e}), expected e.g. \"60s\" or \"10m\"")
        })
    }
}

impl fmt::Display for Dur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", humantime::format_duration(self.0))
    }
}

/// Fire when a value stays above `above` for at least `for`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRule {
    pub above: Percent,
    #[serde(rename = "for")]
    pub duration: Dur,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_parses() {
        assert_eq!(Percent::try_from("85%".to_owned()).unwrap().0, 85.0);
        assert_eq!(Percent::try_from("0.5%".to_owned()).unwrap().0, 0.5);
    }

    #[test]
    fn percent_rejects_bad_input() {
        assert!(Percent::try_from("85".to_owned()).is_err());
        assert!(Percent::try_from("abc%".to_owned()).is_err());
        assert!(Percent::try_from("101%".to_owned()).is_err());
        assert!(Percent::try_from("-1%".to_owned()).is_err());
    }

    #[test]
    fn dur_parses() {
        assert_eq!(
            Dur::try_from("10m".to_owned()).unwrap().get(),
            Duration::from_secs(600)
        );
        assert_eq!(
            Dur::try_from("14d".to_owned()).unwrap().get(),
            Duration::from_secs(14 * 86400)
        );
        assert!(Dur::try_from("10".to_owned()).is_err());
    }
}
