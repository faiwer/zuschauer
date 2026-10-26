mod alerts;
mod collect;
mod config;
mod cron;
mod heartbeat;
mod http;
mod jobs;
mod model;
mod notify;
mod state;
mod store;
mod web;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: zuschauer [--check] [CONFIG]\n       \
    zuschauer --state [--only PARTS] [CONFIG]\n       zuschauer --sensors\n\n  \
    CONFIG        path to the config file (default: config.jsonc)\n  \
    --check       validate the config, print it with defaults applied, and exit\n  \
    --state       read everything the config watches once, print it, and exit\n  \
    --only PARTS  with --state, only these, comma-separated: host, containers,\n                \
    checks, certificates, services\n  \
    --sensors     list temperature sensors with their current readings, and exit";

fn main() -> ExitCode {
    let mut check = false;
    let mut state = false;
    let mut parts = None;
    let mut path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--state" => state = true,
            "--only" => match args.next().as_deref().map(state::parse_parts) {
                Some(Ok(only)) => parts = Some(only),
                Some(Err(e)) => return usage_error(&e),
                None => return usage_error(&format!("--only needs some of {}", state::NAMES)),
            },
            "--sensors" => return collect::list_sensors(),
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ if arg.starts_with('-') || path.is_some() => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            _ => path = Some(PathBuf::from(arg)),
        }
    }
    if parts.is_some() && !state {
        return usage_error("--only works with --state");
    }
    if check && state {
        return usage_error("--check and --state don't go together");
    }
    let path = path.unwrap_or_else(|| PathBuf::from("config.jsonc"));

    let config = match config::load(&path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    if check {
        print!("{config}");
        return ExitCode::SUCCESS;
    }
    if state {
        state::show(config, parts.unwrap_or(collect::Parts::ALL));
    }

    run(config)
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}\n\n{USAGE}");
    ExitCode::from(2)
}

#[tokio::main(flavor = "current_thread")]
async fn run(config: &'static config::Config) -> ExitCode {
    notify::spawn(config);
    collect::spawn(config, collect::Parts::ALL);
    jobs::spawn(&config.apps);
    heartbeat::spawn(config.heartbeat.as_ref());
    match web::serve(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("web server on {}: {e}", config.listen);
            ExitCode::FAILURE
        }
    }
}
