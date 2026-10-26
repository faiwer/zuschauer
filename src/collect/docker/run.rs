use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, sleep};

use crate::config::{App, DockerConfig};

use super::api::{Api, find_socket};
use super::apps::observe_app_containers;
use super::tracker::Tracker;

/// The first CPU values are averaged over a shorter window so they show up
/// right after start instead of one full `every` later.
const FIRST_WINDOW: Duration = Duration::from_secs(1);

pub async fn run(config: &'static DockerConfig, apps: &'static [App]) {
    let api = find_socket(config.socket.as_deref()).map(Api::new);
    match &api {
        Some(api) => eprintln!("docker: API at {}", api.socket().display()),
        None if cfg!(target_os = "freebsd") => eprintln!("docker: containers are jails"),
        None => eprintln!("docker: no API socket found, containers are shown by ID"),
    }
    let mut tracker = Tracker::default();
    if tracker.tick(api.as_ref()).await {
        observe_app_containers(apps);
    }
    sleep(FIRST_WINDOW.min(config.every)).await;

    let mut ticker = interval(config.every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        if tracker.tick(api.as_ref()).await {
            observe_app_containers(apps);
        }
    }
}
