use tokio::time::{MissedTickBehavior, interval};

use crate::alerts::{Observation, observe};
use crate::config::{App, AppCommand, Dur};
use crate::model::{EntityId, MetricKind, Sample, now};
use crate::store::store;

use super::super::outcome::{Verdict, record, record_verdict};
use super::process::{End, Finished, execute};

/// What a finished run means, with the text to show for it.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The output.
    Fine(String),
    /// Exited with a `warnOn` code: why, with the output after it.
    Warning(String),
    /// Why it failed, with the output after it.
    Failed(String),
}

/// Runs never overlap: a run still going when the next one is due makes it
/// skip to the following tick.
pub async fn run(app: &'static App, command: &'static AppCommand) {
    let entity = EntityId::Command {
        app: app.id.clone(),
        command: command.name.clone(),
    };
    let label = format!("{}/{}", app.id, command.name);
    let mut status = None;

    let mut ticker = interval(command.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        store().set_running_since(entity.clone(), Some(now()));
        let finished = execute(command, || {
            let problem = format!("still running after {}", Dur::from(command.timeout));
            eprintln!("command {label}: {problem}");
            status = Some(record(&entity, status, Err(problem.clone())));
            if let Some(alert) = &command.alert {
                observe(alert, Observation::Problem(problem));
            }
        })
        .await;

        let outcome = outcome(&finished, command);
        {
            let mut store = store();
            store.set_running_since(entity.clone(), None);
            if !matches!(finished.end, End::Failed(_)) {
                let took = Sample::now(finished.took.as_secs_f32());
                store.push(entity.clone(), MetricKind::Runtime, took);
            }
            let message = match &outcome {
                Outcome::Fine(output) => Some(output.clone()),
                Outcome::Warning(_) | Outcome::Failed(_) => None,
            };
            store.set_message(entity.clone(), message);
        }
        let (verdict, error, observation) = match outcome {
            Outcome::Fine(_) => (Verdict::Up, None, Observation::Fine),
            Outcome::Warning(error) => {
                eprintln!("command {label}: warning, {error}");
                (
                    Verdict::Warn,
                    Some(error.clone()),
                    Observation::Warning(error),
                )
            }
            Outcome::Failed(error) => {
                eprintln!("command {label}: {error}");
                (
                    Verdict::Down,
                    Some(error.clone()),
                    Observation::Problem(error),
                )
            }
        };
        status = Some(record_verdict(&entity, status, verdict, error));
        if let Some(alert) = &command.alert {
            observe(alert, observation);
        }
    }
}

fn outcome(finished: &Finished, command: &AppCommand) -> Outcome {
    let (reason, warning) = match &finished.end {
        End::Code(0) => return Outcome::Fine(finished.output.clone()),
        End::Code(code) => (format!("exit {code}"), command.warn_on.contains(code)),
        End::Signal(signal) => (format!("killed by signal {signal}"), false),
        End::Killed => (
            format!("no answer within {}, killed", Dur::from(command.timeout)),
            false,
        ),
        End::Failed(error) => return Outcome::Failed(error.clone()),
    };
    let text = match finished.output.as_str() {
        "" => reason,
        output => format!("{reason}: {output}"),
    };
    if warning {
        Outcome::Warning(text)
    } else {
        Outcome::Failed(text)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::{OutputFrom, OutputSlice};

    fn finished(end: End, output: &str) -> Finished {
        Finished {
            end,
            output: output.to_owned(),
            took: Duration::ZERO,
        }
    }

    #[test]
    fn describes_outcomes() {
        let command = AppCommand {
            name: "check".to_owned(),
            run: vec!["check".to_owned()],
            every: Duration::from_secs(60),
            timeout: Duration::from_secs(30),
            kill_on_timeout: true,
            warn_on: vec![1],
            output: OutputSlice {
                from: OutputFrom::Tail,
                lines: 1,
                max_line_length: 512,
            },
            alert: None,
        };
        let outcome = |end, output| outcome(&finished(end, output), &command);
        let failed = |text: &str| Outcome::Failed(text.to_owned());
        assert_eq!(
            outcome(End::Code(0), "SMART OK"),
            Outcome::Fine("SMART OK".to_owned())
        );
        assert_eq!(
            outcome(End::Code(1), "DISK WARNING"),
            Outcome::Warning("exit 1: DISK WARNING".to_owned())
        );
        assert_eq!(
            outcome(End::Code(2), "DISK CRITICAL"),
            failed("exit 2: DISK CRITICAL")
        );
        assert_eq!(outcome(End::Code(3), ""), failed("exit 3"));
        assert_eq!(outcome(End::Signal(9), ""), failed("killed by signal 9"));
        assert_eq!(
            outcome(End::Killed, "scanning"),
            failed("no answer within 30s, killed: scanning")
        );
        assert_eq!(
            outcome(End::Failed("x: not found".to_owned()), "ignored"),
            failed("x: not found")
        );
    }
}
