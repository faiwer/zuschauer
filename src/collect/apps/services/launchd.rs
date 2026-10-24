use super::unit::Unit;

#[cfg(target_os = "macos")]
use std::collections::HashMap;

#[cfg(target_os = "macos")]
use super::command::output;

/// `launchctl print`'s exit code for a service that isn't loaded.
#[cfg(target_os = "macos")]
const NOT_FOUND: i32 = 113;

/// Asks launchd about each service with `launchctl print`, one call per
/// service: it can't print several.
#[cfg(target_os = "macos")]
#[derive(Default)]
pub(super) struct Reader {
    /// Name → the target that found it, e.g. `gui/501/<label>`, so a system
    /// service isn't looked up in the user's domain first every time.
    targets: HashMap<String, String>,
}

#[cfg(target_os = "macos")]
impl Reader {
    pub fn new() -> Self {
        Self::default()
    }

    /// In the order of `names`.
    pub async fn read(&mut self, names: &[&str]) -> Vec<Result<Unit, String>> {
        let mut units = Vec::with_capacity(names.len());
        for name in names {
            units.push(self.read_one(name).await);
        }
        units
    }

    async fn read_one(&mut self, name: &str) -> Result<Unit, String> {
        let candidates = match self.targets.remove(name) {
            Some(target) => vec![target],
            // SAFETY: getuid has no preconditions and can't fail.
            None => targets(name, unsafe { libc::getuid() }),
        };
        for target in candidates {
            let out = output("launchctl", &["print", &target]).await?;
            match out.code {
                Some(0) => {
                    self.targets.insert(name.to_owned(), target);
                    return Ok(parse(&out.stdout));
                }
                Some(NOT_FOUND) => {}
                _ => return Err(format!("launchctl print {target}: {}", out.stderr.trim())),
            }
        }
        Ok(Unit {
            problem: Some("not found".to_owned()),
            restarts: 0,
        })
    }
}

/// Where to look for `name`: as it is when it names a domain, otherwise in
/// the user's own domain, then in the system's.
fn targets(name: &str, uid: u32) -> Vec<String> {
    if ["system/", "gui/", "user/"]
        .iter()
        .any(|domain| name.starts_with(domain))
    {
        vec![name.to_owned()]
    } else {
        vec![format!("gui/{uid}/{name}"), format!("system/{name}")]
    }
}

/// `launchctl print` lists `key = value` lines, indented by one tab at the
/// top level and deeper inside nested blocks. Apple doesn't promise this
/// format, so anything missing reads as healthy with no restarts.
fn parse(text: &str) -> Unit {
    let field = |key: &str| {
        text.lines().find_map(|line| {
            line.strip_prefix('\t')
                .filter(|line| !line.starts_with('\t'))?
                .strip_prefix(key)?
                .strip_prefix(" = ")
        })
    };
    let keepalive = field("properties")
        .is_some_and(|properties| properties.split('|').any(|p| p.trim() == "keepalive"));
    let state = field("state").unwrap_or_default();
    let last = match (field("last terminating signal"), field("last exit code")) {
        (Some(signal), _) => Some(format!("killed by {signal}")),
        (None, Some(code)) if code != "0" && code != "(never exited)" => {
            Some(format!("exit code {code}"))
        }
        _ => None,
    };
    let with_last = |state: &str| match &last {
        Some(last) => format!("{state}, {last}"),
        None => state.to_owned(),
    };
    let problem = match state {
        "running" => None,
        // Meant to stay up: launchd will start it again.
        "spawn scheduled" if keepalive => Some(with_last("restarting")),
        _ if keepalive => Some(with_last("not running")),
        // Runs on demand; only a failed run is a problem.
        _ => last.map(|last| format!("last run failed, {last}")),
    };
    let runs: u64 = field("runs")
        .and_then(|runs| runs.parse().ok())
        .unwrap_or(0);
    Unit {
        problem,
        // Every launch is a run, the first one too.
        restarts: runs.saturating_sub(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print(fields: &[(&str, &str)]) -> String {
        let mut text = "gui/501/dev.example.test = {\n\tactive count = 1\n".to_owned();
        for (key, value) in fields {
            text.push_str(&format!("\t{key} = {value}\n"));
        }
        text.push_str("\tenvironment = {\n\t\tstate = nested, ignored\n\t}\n}\n");
        text
    }

    #[test]
    fn reads_a_keepalive_job() {
        let running = print(&[
            ("state", "running"),
            ("runs", "3"),
            ("last exit code", "3"),
            ("properties", "keepalive | inferred program"),
        ]);
        assert_eq!(
            parse(&running),
            Unit {
                problem: None,
                restarts: 2
            }
        );

        let killed = print(&[
            ("state", "spawn scheduled"),
            ("runs", "3"),
            ("last terminating signal", "Killed: 9"),
            ("properties", "keepalive | inferred program"),
        ]);
        assert_eq!(
            parse(&killed).problem.as_deref(),
            Some("restarting, killed by Killed: 9")
        );

        let stopped = print(&[("state", "not running"), ("properties", "keepalive")]);
        assert_eq!(parse(&stopped).problem.as_deref(), Some("not running"));
    }

    #[test]
    fn reads_an_on_demand_job() {
        let idle = print(&[
            ("state", "not running"),
            ("runs", "27"),
            ("last exit code", "0"),
            ("properties", "supports transactions | system service"),
        ]);
        assert_eq!(
            parse(&idle),
            Unit {
                problem: None,
                restarts: 26
            }
        );

        let failed = print(&[("state", "not running"), ("last exit code", "1")]);
        assert_eq!(
            parse(&failed).problem.as_deref(),
            Some("last run failed, exit code 1")
        );

        let never = print(&[
            ("state", "not running"),
            ("last exit code", "(never exited)"),
        ]);
        assert_eq!(parse(&never).problem, None);
    }

    #[test]
    fn looks_in_the_user_domain_first() {
        assert_eq!(
            targets("dev.example.test", 501),
            ["gui/501/dev.example.test", "system/dev.example.test"]
        );
        assert_eq!(
            targets("system/dev.example.test", 501),
            ["system/dev.example.test"]
        );
    }
}
