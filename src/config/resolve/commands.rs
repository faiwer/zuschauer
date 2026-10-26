use std::collections::BTreeMap;
use std::time::Duration;

use crate::config::raw::{RawCommand, RawOutput};
use crate::config::resolved::{AppCommand, OutputSlice};
use crate::config::types::Dur;

use super::alerts::{AlertNames, state_alert};
use super::every::{DEFAULT_EVERY, every};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_LINES: usize = 1;
const MAX_LINES: usize = 50;
const DEFAULT_MAX_LINE_LENGTH: usize = 512;
const MAX_LINE_LENGTH: usize = 10_000;

/// `app_at` is the app's config path, e.g. `apps.web`.
pub(super) fn app_commands(
    app_at: &str,
    app_id: &str,
    app_name: &str,
    raw: BTreeMap<String, RawCommand>,
    errors: &mut Vec<String>,
) -> Vec<AppCommand> {
    raw.into_iter()
        .map(|(name, raw)| {
            let at = format!("{app_at}.commands.\"{name}\"");
            if name.is_empty() {
                errors.push(format!("{at}: a command name can't be empty"));
            }
            let names = AlertNames::new(
                format!("{app_id}/{name}"),
                format!("{app_name} command {name}"),
            );
            app_command(&at, name, raw, names, errors)
        })
        .collect()
}

fn app_command(
    at: &str,
    name: String,
    raw: RawCommand,
    names: AlertNames,
    errors: &mut Vec<String>,
) -> AppCommand {
    if raw.run.first().is_none_or(String::is_empty) {
        errors.push(format!(
            "{at}.run: needs the program first, then its arguments"
        ));
    }
    let every = every(at, raw.every, DEFAULT_EVERY, errors);
    let timeout = raw.timeout.map_or(DEFAULT_TIMEOUT, Dur::get);
    if timeout.is_zero() {
        errors.push(format!("{at}.timeout: must be more than 0"));
    }
    AppCommand {
        name,
        run: raw.run,
        every,
        timeout,
        kill_on_timeout: raw.kill_on_timeout.unwrap_or(true),
        warn_on: warn_on(at, raw.warn_on.unwrap_or_default(), errors),
        output: output(at, raw.output.unwrap_or_default(), errors),
        alert: raw.alert.map(|alert| state_alert(at, alert, names, errors)),
    }
}

/// Exit codes are 0 to 255, and 0 is success.
fn warn_on(at: &str, raw: Vec<i64>, errors: &mut Vec<String>) -> Vec<i32> {
    raw.into_iter()
        .filter_map(|code| match i32::try_from(code) {
            Ok(code @ 1..=255) => Some(code),
            _ => {
                errors.push(format!(
                    "{at}.warnOn: {code} isn't a failing exit code, expected 1 to 255"
                ));
                None
            }
        })
        .collect()
}

fn output(at: &str, raw: RawOutput, errors: &mut Vec<String>) -> OutputSlice {
    let lines = raw.lines.unwrap_or(DEFAULT_LINES);
    if !(1..=MAX_LINES).contains(&lines) {
        errors.push(format!("{at}.output.lines: expected 1 to {MAX_LINES}"));
    }
    let max_line_length = raw.max_line_length.unwrap_or(DEFAULT_MAX_LINE_LENGTH);
    if !(1..=MAX_LINE_LENGTH).contains(&max_line_length) {
        errors.push(format!(
            "{at}.output.maxLineLength: expected 1 to {MAX_LINE_LENGTH}"
        ));
    }
    OutputSlice {
        from: raw.position.unwrap_or_default(),
        lines,
        max_line_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFrom;
    use crate::config::raw::RawConfig;
    use crate::config::resolve::resolve;

    fn parse(json: &str) -> RawConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn applies_command_defaults() {
        let config = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "nas": { "name": "NAS", "commands": {
                    "smart": { "run": ["check_smart", "-d", "/dev/ada0"], "alert": {} },
                    "backup": {
                        "run": ["/usr/local/bin/check_backup"],
                        "every": "1h",
                        "timeout": "2h",
                        "killOnTimeout": false,
                        "warnOn": [1, 24],
                        "output": { "position": "head", "lines": 3, "maxLineLength": 80 }
                    }
                } } }
            }"#,
        ))
        .unwrap();
        let [backup, smart] = &config.apps[0].commands[..] else {
            panic!("{:#?}", config.apps[0].commands);
        };
        assert_eq!(smart.run, ["check_smart", "-d", "/dev/ada0"]);
        assert_eq!(
            (smart.every, smart.timeout),
            (DEFAULT_EVERY, DEFAULT_TIMEOUT)
        );
        assert!(smart.kill_on_timeout);
        assert!(smart.warn_on.is_empty());
        assert_eq!(backup.warn_on, [1, 24]);
        assert_eq!(smart.output.from, OutputFrom::Tail);
        assert_eq!((smart.output.lines, smart.output.max_line_length), (1, 512));
        let alert = smart.alert.as_ref().unwrap();
        assert_eq!(
            (alert.id.as_str(), alert.label.as_str()),
            ("nas/smart", "NAS command smart")
        );
        assert!(!backup.kill_on_timeout);
        assert_eq!(backup.timeout, Duration::from_secs(7200));
        assert_eq!(backup.output.from, OutputFrom::Head);
        assert_eq!(
            (backup.output.lines, backup.output.max_line_length),
            (3, 80)
        );
    }

    #[test]
    fn checks_commands() {
        let errors = resolve(parse(
            r#"{
                "listen": "127.0.0.1:9100",
                "apps": { "nas": { "commands": {
                    "": { "run": ["true"] },
                    "none": { "run": [] },
                    "blank": { "run": [""] },
                    "bad": { "run": ["x"], "timeout": "0s", "output": { "lines": 0, "maxLineLength": 0 } },
                    "codes": { "run": ["x"], "warnOn": [0, 1, 256] }
                } } }
            }"#,
        ))
        .unwrap_err();
        // Empty name, two bad `run`s, zero timeout, lines, maxLineLength,
        // warnOn 0 and 256.
        assert_eq!(errors.len(), 8, "{errors:#?}");
    }
}
