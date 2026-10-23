use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

use crate::config::{Config, Dur, Notifier, NotifierKind};

use super::message::Message;
use super::ntfy;

/// Messages waiting per notifier; more are dropped while it's unreachable.
const QUEUE: usize = 64;
const TIMEOUT: Duration = Duration::from_secs(15);
const FIRST_RETRY: Duration = Duration::from_secs(5);
const MAX_RETRY: Duration = Duration::from_secs(5 * 60);

/// Why a message wasn't delivered.
pub(super) enum Failure {
    /// The service may accept it later.
    Retry(String),
    /// The service refused it; sending it again won't help.
    Rejected(String),
}

struct Queue {
    name: &'static str,
    tx: mpsc::Sender<Message>,
}

static QUEUES: OnceLock<Vec<Queue>> = OnceLock::new();

/// Starts one delivery task per notifier.
pub fn spawn(config: &'static Config) {
    if config.notifiers.is_empty() && config.alerts().next().is_some() {
        eprintln!("alerts: no notifiers configured, alerts only show on the page");
    }
    let queues = config
        .notifiers
        .iter()
        .map(|notifier| {
            let (tx, rx) = mpsc::channel(QUEUE);
            tokio::spawn(deliver(notifier, &config.context, rx));
            Queue {
                name: &notifier.name,
                tx,
            }
        })
        .collect();
    let _ = QUEUES.set(queues);
}

/// Queues `message` for every notifier.
pub fn send(message: Message) {
    for queue in QUEUES.get().into_iter().flatten() {
        if queue.tx.try_send(message.clone()).is_err() {
            eprintln!(
                "notifier {}: queue full, dropped \"{}\"",
                queue.name, message.title
            );
        }
    }
}

/// One message at a time, in order: a message that can't be delivered yet
/// holds back the ones after it.
async fn deliver(
    notifier: &'static Notifier,
    context: &'static str,
    mut rx: mpsc::Receiver<Message>,
) {
    let name = &notifier.name;
    while let Some(message) = rx.recv().await {
        let mut delay = FIRST_RETRY;
        loop {
            let result = timeout(TIMEOUT, publish(notifier, context, &message))
                .await
                .unwrap_or_else(|_| {
                    Err(Failure::Retry(format!(
                        "no response within {}",
                        Dur::from(TIMEOUT)
                    )))
                });
            match result {
                Ok(()) => {
                    eprintln!("notifier {name}: sent \"{}\"", message.title);
                    break;
                }
                Err(Failure::Rejected(reason)) => {
                    eprintln!("notifier {name}: dropped \"{}\", {reason}", message.title);
                    break;
                }
                Err(Failure::Retry(reason)) => {
                    eprintln!(
                        "notifier {name}: {reason}, retrying in {}",
                        Dur::from(delay)
                    );
                    sleep(delay).await;
                    delay = (delay * 2).min(MAX_RETRY);
                }
            }
        }
    }
}

async fn publish(notifier: &Notifier, context: &str, message: &Message) -> Result<(), Failure> {
    match &notifier.kind {
        NotifierKind::Ntfy(config) => ntfy::publish(config, context, message).await,
    }
}
