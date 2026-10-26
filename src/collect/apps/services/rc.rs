use super::unit::Unit;

#[cfg(target_os = "freebsd")]
use super::command::output;

/// Asks each rc.d script with `service <name> onestatus`, one call per
/// service. `onestatus` answers whether or not the service is enabled in
/// `rc.conf`, where `status` would refuse.
#[cfg(target_os = "freebsd")]
#[derive(Default)]
pub(super) struct Reader;

#[cfg(target_os = "freebsd")]
impl Reader {
    pub fn new() -> Self {
        Self
    }

    /// In the order of `names`.
    pub async fn read(&mut self, names: &[&str]) -> Vec<Result<Unit, String>> {
        let mut units = Vec::with_capacity(names.len());
        for name in names {
            let unit = output("service", &[name, "onestatus"])
                .await
                .and_then(|out| parse(out.code, &format!("{}{}", out.stdout, out.stderr)));
            units.push(unit);
        }
        units
    }
}

/// rc.d exits with 1 for stopped, missing and unsupported alike, so the text
/// tells them apart. rc.d doesn't restart services, so there are no restarts.
fn parse(code: Option<i32>, text: &str) -> Result<Unit, String> {
    let text = text.trim();
    let problem = |problem: &str| {
        Ok(Unit {
            problem: Some(problem.to_owned()),
            restarts: 0,
        })
    };
    if code == Some(0) {
        return Ok(Unit {
            problem: None,
            restarts: 0,
        });
    }
    // A pid file only root can read makes a running service look stopped.
    if let Some(denied) = text.lines().find(|line| line.contains("Permission denied")) {
        return Err(format!("can't check it: {}", denied.trim()));
    }
    if text.contains("unknown directive 'status'") {
        return Err("its rc.d script has no status command".to_owned());
    }
    if text.contains("does not exist in /etc/rc.d") {
        return problem("not found");
    }
    if text.ends_with("is not running.") || text.is_empty() {
        return problem("stopped");
    }
    // A script's own status command, with its own message.
    problem(&format!("stopped, {}", first_line(text)))
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default().trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_of(code: Option<i32>, text: &str) -> Result<Option<String>, String> {
        parse(code, text).map(|unit| unit.problem)
    }

    #[test]
    fn reads_running_and_stopped() {
        assert_eq!(
            problem_of(Some(0), "sshd is running as pid 3894.\n"),
            Ok(None)
        );
        assert_eq!(
            problem_of(Some(1), "ntpd is not running.\n"),
            Ok(Some("stopped".to_owned()))
        );
        assert_eq!(
            problem_of(
                Some(1),
                "nosuchsvc does not exist in /etc/rc.d or the local startup\ndirectories (/usr/local/etc/rc.d), or is not executable\n"
            ),
            Ok(Some("not found".to_owned()))
        );
        assert_eq!(
            problem_of(Some(3), "database unreachable\n"),
            Ok(Some("stopped, database unreachable".to_owned()))
        );
        assert_eq!(problem_of(None, ""), Ok(Some("stopped".to_owned())));
    }

    #[test]
    fn errors_when_it_cant_tell() {
        assert_eq!(
            problem_of(
                Some(1),
                "cron is not running.\neval: cannot open /var/run/cron.pid: Permission denied\n"
            ),
            Err(
                "can't check it: eval: cannot open /var/run/cron.pid: Permission denied".to_owned()
            )
        );
        assert_eq!(
            problem_of(
                Some(1),
                "/etc/rc.d/hostname: unknown directive 'status'.\nUsage: /etc/rc.d/hostname [fast|force|one|quiet](start|stop)\n"
            ),
            Err("its rc.d script has no status command".to_owned())
        );
    }
}
