use std::collections::HashMap;

use super::unit::Unit;

#[cfg(target_os = "linux")]
use super::command::output;

#[cfg(target_os = "linux")]
const PROPERTIES: &str = "LoadState,ActiveState,SubState,Type,Result,NRestarts,ExecMainStatus";

/// Asks systemd about all services in one `systemctl show` call.
#[cfg(target_os = "linux")]
#[derive(Default)]
pub(super) struct Reader;

#[cfg(target_os = "linux")]
impl Reader {
    pub fn new() -> Self {
        Self
    }

    /// In the order of `names`.
    pub async fn read(&mut self, names: &[&str]) -> Vec<Result<Unit, String>> {
        let units: Vec<String> = names.iter().map(|name| unit_name(name)).collect();
        let mut args = vec!["show", "--property", PROPERTIES, "--"];
        args.extend(units.iter().map(String::as_str));
        let result = output("systemctl", &args).await.and_then(|out| {
            if out.code != Some(0) {
                return Err(format!("systemctl: {}", out.stderr.trim()));
            }
            parse_all(&out.stdout, names.len())
        });
        match result {
            Ok(units) => units.into_iter().map(Ok).collect(),
            Err(error) => names.iter().map(|_| Err(error.clone())).collect(),
        }
    }
}

/// `.service` is optional in the config.
fn unit_name(name: &str) -> String {
    if name.ends_with(".service") {
        name.to_owned()
    } else {
        format!("{name}.service")
    }
}

/// `systemctl show` prints a block of `Key=value` lines per unit, in the
/// order asked, separated by blank lines.
fn parse_all(text: &str, count: usize) -> Result<Vec<Unit>, String> {
    let units: Vec<Unit> = text
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .map(parse)
        .collect();
    if units.len() != count {
        return Err(format!(
            "systemctl: asked about {count} units, got {}",
            units.len()
        ));
    }
    Ok(units)
}

fn parse(block: &str) -> Unit {
    let properties: HashMap<&str, &str> = block
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    let get = |key: &str| properties.get(key).copied().unwrap_or_default();
    Unit {
        problem: problem(get),
        restarts: get("NRestarts").parse().unwrap_or(0),
    }
}

fn problem<'a>(get: impl Fn(&str) -> &'a str) -> Option<String> {
    match get("LoadState") {
        "loaded" => {}
        "not-found" => return Some("not found".to_owned()),
        other => return Some(format!("not loaded, {other}")),
    }
    let last = last_exit(get("Result"), get("ExecMainStatus"));
    let with_last = |state: &str| match &last {
        Some(last) => format!("{state}, {last}"),
        None => state.to_owned(),
    };
    match (get("ActiveState"), get("SubState")) {
        ("failed", _) => Some(with_last("failed")),
        // Runs and exits; only a failed run is a problem.
        _ if get("Type") == "oneshot" => None,
        ("active" | "reloading" | "refreshing", _) => None,
        (_, "auto-restart") => Some(with_last("restarting")),
        ("activating", _) => Some("starting".to_owned()),
        ("deactivating", _) => Some("stopping".to_owned()),
        _ => Some("stopped".to_owned()),
    }
}

/// How the last run ended, when that wasn't cleanly. `status` is the exit
/// code or the signal number, depending on `result`.
fn last_exit(result: &str, status: &str) -> Option<String> {
    match result {
        "" | "success" => None,
        "exit-code" => Some(format!("exit code {status}")),
        "signal" | "core-dump" => Some(format!("killed by {}", signal_name(status))),
        "oom-kill" => Some("killed for running out of memory".to_owned()),
        other => Some(other.replace('-', " ")),
    }
}

fn signal_name(number: &str) -> String {
    let name = match number {
        "1" => "SIGHUP",
        "2" => "SIGINT",
        "6" => "SIGABRT",
        "9" => "SIGKILL",
        "11" => "SIGSEGV",
        "15" => "SIGTERM",
        _ => return format!("signal {number}"),
    };
    name.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(pairs: &[(&str, &str)]) -> String {
        pairs.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
    }

    fn problem_of(pairs: &[(&str, &str)]) -> Option<String> {
        parse(&block(pairs)).problem
    }

    #[test]
    fn splits_blocks_in_order() {
        let text = format!(
            "{}\n{}",
            block(&[
                ("LoadState", "loaded"),
                ("ActiveState", "active"),
                ("NRestarts", "4")
            ]),
            block(&[("LoadState", "not-found"), ("ActiveState", "inactive")]),
        );
        let units = parse_all(&text, 2).unwrap();
        assert_eq!(
            units,
            [
                Unit {
                    problem: None,
                    restarts: 4
                },
                Unit {
                    problem: Some("not found".to_owned()),
                    restarts: 0
                },
            ]
        );
        assert!(parse_all(&text, 3).is_err());
    }

    #[test]
    fn judges_daemons() {
        let loaded = |active: &'static str, sub: &'static str, result: &'static str| {
            problem_of(&[
                ("LoadState", "loaded"),
                ("Type", "simple"),
                ("ActiveState", active),
                ("SubState", sub),
                ("Result", result),
                ("ExecMainStatus", "3"),
            ])
        };
        assert_eq!(loaded("active", "running", "success"), None);
        assert_eq!(
            loaded("failed", "failed", "exit-code").as_deref(),
            Some("failed, exit code 3")
        );
        assert_eq!(
            loaded("activating", "auto-restart", "exit-code").as_deref(),
            Some("restarting, exit code 3")
        );
        assert_eq!(
            loaded("inactive", "dead", "success").as_deref(),
            Some("stopped")
        );
        assert_eq!(
            loaded("failed", "failed", "start-limit-hit").as_deref(),
            Some("failed, start limit hit")
        );
        assert_eq!(
            problem_of(&[("LoadState", "masked")]).as_deref(),
            Some("not loaded, masked")
        );
    }

    #[test]
    fn judges_oneshots_only_by_failures() {
        let oneshot = |active: &'static str, result: &'static str| {
            problem_of(&[
                ("LoadState", "loaded"),
                ("Type", "oneshot"),
                ("ActiveState", active),
                ("SubState", "dead"),
                ("Result", result),
                ("ExecMainStatus", "9"),
            ])
        };
        assert_eq!(oneshot("inactive", "success"), None);
        assert_eq!(
            oneshot("failed", "signal").as_deref(),
            Some("failed, killed by SIGKILL")
        );
    }

    #[test]
    fn adds_the_service_suffix() {
        assert_eq!(unit_name("backup"), "backup.service");
        assert_eq!(unit_name("backup.service"), "backup.service");
    }
}
