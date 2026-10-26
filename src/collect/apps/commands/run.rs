use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval};

use crate::alerts::{Observation, observe};
use crate::config::{App, AppCommand, Dur};
use crate::model::{EntityId, MetricKind, Sample, now};
use crate::store::store;

use super::super::outcome::record;
use super::process::{End, Finished, execute};

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

        let outcome = outcome(&finished, command.timeout);
        {
            let mut store = store();
            store.set_running_since(entity.clone(), None);
            if !matches!(finished.end, End::Failed(_)) {
                let took = Sample::now(finished.took.as_secs_f32());
                store.push(entity.clone(), MetricKind::Runtime, took);
            }
            store.set_message(entity.clone(), outcome.clone().ok());
        }
        if let Err(error) = &outcome {
            eprintln!("command {label}: {error}");
        }
        let observation = match &outcome {
            Ok(_) => Observation::Fine,
            Err(error) => Observation::Problem(error.clone()),
        };
        status = Some(record(&entity, status, outcome.map(drop)));
        if let Some(alert) = &command.alert {
            observe(alert, observation);
        }
    }
}

/// The output on success, or why it failed with the output after it.
fn outcome(finished: &Finished, timeout: Duration) -> Result<String, String> {
    let reason = match &finished.end {
        End::Code(0) => return Ok(finished.output.clone()),
        End::Code(code) => format!("exit {code}"),
        End::Signal(signal) => format!("killed by signal {signal}"),
        End::Killed => format!("no answer within {}, killed", Dur::from(timeout)),
        End::Failed(error) => return Err(error.clone()),
    };
    match finished.output.as_str() {
        "" => Err(reason),
        output => Err(format!("{reason}: {output}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(end: End, output: &str) -> Finished {
        Finished {
            end,
            output: output.to_owned(),
            took: Duration::ZERO,
        }
    }

    #[test]
    fn describes_outcomes() {
        let timeout = Duration::from_secs(30);
        let outcome = |end, output| outcome(&finished(end, output), timeout);
        assert_eq!(outcome(End::Code(0), "SMART OK"), Ok("SMART OK".to_owned()));
        assert_eq!(
            outcome(End::Code(2), "DISK CRITICAL"),
            Err("exit 2: DISK CRITICAL".to_owned())
        );
        assert_eq!(outcome(End::Code(1), ""), Err("exit 1".to_owned()));
        assert_eq!(
            outcome(End::Signal(9), ""),
            Err("killed by signal 9".to_owned())
        );
        assert_eq!(
            outcome(End::Killed, "scanning"),
            Err("no answer within 30s, killed: scanning".to_owned())
        );
        assert_eq!(
            outcome(End::Failed("x: not found".to_owned()), "ignored"),
            Err("x: not found".to_owned())
        );
    }
}
