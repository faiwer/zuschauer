use tokio::time::{MissedTickBehavior, interval};

use crate::alerts::{Observation, observe};
use crate::config::{App, AppService, Dur, ServicesConfig};
use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp, now};
use crate::store::store;

use super::super::outcome::record;
use super::restarts::RestartWindow;
use super::unit::Unit;

#[cfg(target_os = "macos")]
use super::launchd::Reader;
#[cfg(target_os = "linux")]
use super::systemd::Reader;

struct Watched {
    app: &'static App,
    service: &'static AppService,
    entity: EntityId,
    status: Option<Status>,
    restarts: RestartWindow,
}

/// One task for all services, since systemd answers for all of them in one
/// call.
pub async fn run(config: &'static ServicesConfig, apps: &'static [App]) {
    let mut watched: Vec<Watched> = apps
        .iter()
        .flat_map(|app| {
            app.services.iter().map(move |service| Watched {
                app,
                service,
                entity: EntityId::Service {
                    app: app.id.clone(),
                    service: service.name.clone(),
                },
                status: None,
                restarts: RestartWindow::default(),
            })
        })
        .collect();
    let names: Vec<&str> = watched.iter().map(|w| w.service.name.as_str()).collect();
    let mut reader = Reader::new();

    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let units = reader.read(&names).await;
        let at = now();
        for (watched, unit) in watched.iter_mut().zip(units) {
            watched.update(unit, at);
        }
    }
}

impl Watched {
    fn update(&mut self, unit: Result<Unit, String>, at: Timestamp) {
        let observation = match unit {
            Ok(unit) => self.judge(unit, at),
            Err(error) => {
                eprintln!("service {}/{}: {error}", self.app.id, self.service.name);
                Observation::Problem(error)
            }
        };
        let outcome = match &observation {
            Observation::Problem(detail) | Observation::Urgent(detail) => Err(detail.clone()),
            _ => Ok(()),
        };
        self.status = Some(record(&self.entity, self.status, outcome));
        if let Some(alert) = &self.service.alert {
            observe(alert, observation);
        }
    }

    /// Down wins over restarting too often: it's the more specific reason.
    fn judge(&mut self, unit: Unit, at: Timestamp) -> Observation {
        let limit = self.service.alert.as_ref().and_then(|a| a.restarts);
        let restarts = limit.map(|limit| {
            let within = Timestamp::try_from(limit.within.as_secs()).unwrap_or(Timestamp::MAX);
            let count = self.restarts.update(unit.restarts, at, within);
            let sample = Sample {
                at,
                value: count as f32,
            };
            store().push(self.entity.clone(), MetricKind::Restarts, sample);
            (count, limit)
        });
        if let Some(problem) = unit.problem {
            return Observation::Problem(problem);
        }
        match restarts {
            Some((count, limit)) if count > u64::from(limit.above) => Observation::Urgent(format!(
                "restarted {count} times in {}",
                Dur::from(limit.within)
            )),
            _ => Observation::Fine,
        }
    }
}
