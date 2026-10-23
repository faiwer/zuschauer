use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, sleep};

use crate::config::DockerConfig;

use super::api::{Api, find_socket};
use super::tracker::Tracker;

/// The first CPU values are averaged over a shorter window so they show up
/// right after start instead of one full `every` later.
const FIRST_WINDOW: Duration = Duration::from_secs(1);

pub async fn run(config: &'static DockerConfig) {
    let api = find_socket(config.socket.as_deref()).map(Api::new);
    match &api {
        Some(api) => eprintln!("docker: API at {}", api.socket().display()),
        None => eprintln!("docker: no API socket found, containers are shown by ID"),
    }
    let mut tracker = Tracker::default();
    tracker.tick(api.as_ref()).await;
    sleep(FIRST_WINDOW.min(config.every)).await;

    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        tracker.tick(api.as_ref()).await;
    }
}
