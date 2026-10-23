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

impl From<Percent> for f32 {
    fn from(percent: Percent) -> Self {
        percent.0
    }
}

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

/// A temperature in degrees Celsius, written as `"70C"`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize)]
#[serde(try_from = "String")]
pub struct Celsius(f32);

impl TryFrom<String> for Celsius {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let number = s
            .strip_suffix('C')
            .ok_or_else(|| format!("expected a temperature like \"70C\", got \"{s}\""))?;
        let value: f32 = number
            .trim()
            .parse()
            .map_err(|_| format!("\"{s}\" is not a valid temperature"))?;
        if !value.is_finite() || value < -273.15 {
            return Err(format!("\"{s}\" is not a possible temperature"));
        }
        Ok(Self(value))
    }
}

impl From<Celsius> for f32 {
    fn from(celsius: Celsius) -> Self {
        celsius.0
    }
}

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}C", self.0)
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

/// How loudly a firing alert is announced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    #[default]
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
        })
    }
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
    fn celsius_parses() {
        assert_eq!(Celsius::try_from("70C".to_owned()).unwrap().0, 70.0);
        assert_eq!(Celsius::try_from("-5.5C".to_owned()).unwrap().0, -5.5);
        assert!(Celsius::try_from("70".to_owned()).is_err());
        assert!(Celsius::try_from("70F".to_owned()).is_err());
        assert!(Celsius::try_from("infC".to_owned()).is_err());
        assert!(Celsius::try_from("-300C".to_owned()).is_err());
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
