use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::alerts::{Observation, observe};
use crate::config::{App, AppCertificate, CertificatesConfig};
use crate::http::{HostPort, certificate, describe};
use crate::model::{EntityId, Timestamp, now};
use crate::store::store;

use super::expiry::not_after;
use super::outcome::record;

const TIMEOUT: Duration = Duration::from_secs(10);

pub async fn run(
    app: &'static App,
    watched: &'static AppCertificate,
    config: &'static CertificatesConfig,
) {
    let target = &watched.target;
    let entity = EntityId::Certificate {
        app: app.id.clone(),
        host: target.to_string(),
    };
    let label = format!("{}/{target}", app.id);
    let mut status = None;

    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let mut observation = Observation::Fine;
        let outcome = expires_at(target).await.map(|at| {
            let left = Duration::from_secs(at.saturating_sub(now()).into());
            let days = left.as_secs() / 86400;
            let soon = if left < config.warn_before {
                observation = Observation::Problem(format!("expires in {days}d"));
                ", expiring soon"
            } else {
                ""
            };
            eprintln!("certificate {label}: expires in {days}d{soon}");
            store().set_expires_at(entity.clone(), at);
        });
        if let Err(error) = &outcome {
            eprintln!("certificate {label}: {error}");
            observation = Observation::Problem(error.clone());
        }
        status = Some(record(&entity, status, outcome));
        if let Some(alert) = &watched.alert {
            observe(alert, observation);
        }
    }
}

/// When the certificate expires. An expired, untrusted or misnamed
/// certificate fails the handshake and comes back as `Err`.
async fn expires_at(target: &HostPort) -> Result<Timestamp, String> {
    let der = timeout(TIMEOUT, certificate(target))
        .await
        .map_err(|_| format!("no response within {}", humantime::format_duration(TIMEOUT)))?
        .map_err(|e| describe(&e))?;
    let seconds = not_after(&der).ok_or("can't read the certificate's expiry date")?;
    // Certificates valid until 9999 exist; they're simply "far away".
    Ok(Timestamp::try_from(seconds.max(0)).unwrap_or(Timestamp::MAX))
}
