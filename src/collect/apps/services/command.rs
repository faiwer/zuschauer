use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct Output {
    /// `None` when killed by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Runs `program` and collects what it printed. A program that hangs is
/// killed after `TIMEOUT`.
pub(super) async fn output(program: &str, args: &[&str]) -> Result<Output, String> {
    let run = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = timeout(TIMEOUT, run)
        .await
        .map_err(|_| {
            format!(
                "{program}: no answer within {}",
                humantime::format_duration(TIMEOUT)
            )
        })?
        .map_err(|e| format!("{program}: {e}"))?;
    Ok(Output {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
