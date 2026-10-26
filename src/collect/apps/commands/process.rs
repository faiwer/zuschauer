use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::net::unix::pipe;
use tokio::process::Command;
use tokio::time::{sleep, timeout};

use crate::config::AppCommand;

use super::output::Output;

/// How long we keep reading after the program exits. We need to read a bit
/// more: its last output may not be delivered yet. But we can't wait for the
/// pipe to close: if it started a process in the background, that process keeps
/// the pipe open. E.g. `sh -c "start-daemon & echo started"` exits at once, but
/// the daemon may run for days.
const DRAIN: Duration = Duration::from_secs(1);

pub(super) struct Finished {
    pub end: End,
    /// The slice of stdout and stderr the command keeps.
    pub output: String,
    pub took: Duration,
}

pub(super) enum End {
    Code(i32),
    Signal(i32),
    /// Killed after its timeout.
    Killed,
    /// Zuschauer couldn't run it or wait for it, most often because the
    /// program doesn't exist.
    Failed(String),
}

/// Runs `command`. Its stdout and stderr go into one pipe, so their lines
/// stay in order. When the timeout runs out, the command and everything it
/// started are killed. With `killOnTimeout: false`, `on_timeout` is called
/// instead, and the command keeps running.
pub(super) async fn execute(command: &AppCommand, on_timeout: impl FnOnce()) -> Finished {
    let started = Instant::now();
    let mut output = Output::new(command.output);
    let end = match supervise(command, &mut output, on_timeout).await {
        Ok(end) => end,
        Err(e) => End::Failed(format!("{}: {e}", command.run[0])),
    };
    Finished {
        end,
        output: output.finish(),
        took: started.elapsed(),
    }
}

async fn supervise(
    command: &AppCommand,
    output: &mut Output,
    on_timeout: impl FnOnce(),
) -> io::Result<End> {
    let (reader, writer) = io::pipe()?;
    let mut child = {
        let mut process = Command::new(&command.run[0]);
        process
            .args(&command.run[1..])
            .stdin(Stdio::null())
            .stdout(writer.try_clone()?)
            .stderr(writer)
            .process_group(0)
            .kill_on_drop(true);
        process.spawn()?
    };
    // Dropping `process` closed this side's copies of the write end, so the
    // pipe ends once the program and whatever it started are done with it.
    let mut pipe = pipe::Receiver::from_owned_fd(OwnedFd::from(reader))?;
    let group = child.id().and_then(|id| i32::try_from(id).ok());

    let mut buffer = [0u8; 8192];
    let mut reading = true;
    let mut on_timeout = Some(on_timeout);
    let mut killed = false;
    let deadline = sleep(command.timeout);
    tokio::pin!(deadline);
    let status = loop {
        tokio::select! {
            status = child.wait() => break status?,
            read = pipe.read(&mut buffer), if reading => match read {
                // 0 bytes received = end of output.
                Ok(0) | Err(_) => reading = false,
                Ok(n) => output.push(&buffer[..n]),
            },
            () = &mut deadline, if on_timeout.is_some() => {
                let on_timeout = on_timeout.take();
                if command.kill_on_timeout {
                    killed = true;
                    kill(group);
                } else if let Some(on_timeout) = on_timeout {
                    on_timeout();
                }
            }
        }
    };

    // Read the rest of the output within the drain period.
    let _ = timeout(DRAIN, async {
        while reading {
            match pipe.read(&mut buffer).await {
                Ok(0) | Err(_) => reading = false,
                Ok(n) => output.push(&buffer[..n]),
            }
        }
    })
    .await;
    Ok(end(status, killed))
}

fn end(status: ExitStatus, killed: bool) -> End {
    match (status.code(), status.signal()) {
        _ if killed => End::Killed,
        (Some(code), _) => End::Code(code),
        (None, Some(signal)) => End::Signal(signal),
        // Never happens, just to make the compiler happy.
        (None, None) => End::Code(-1),
    }
}

/// The whole process group, so nothing the program started lives on.
fn kill(group: Option<i32>) {
    if let Some(group) = group {
        // SAFETY: kill has no memory-safety preconditions; a negative pid
        // names the process group the child leads.
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }
}
