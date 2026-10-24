mod alerts;
mod collect;
mod config;
mod cron;
mod http;
mod jobs;
mod model;
mod notify;
mod store;
mod web;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: zuschauer [--check] [CONFIG]\n       zuschauer --sensors\n\n  \
    CONFIG      path to the config file (default: config.jsonc)\n  \
    --check     validate the config, print it with defaults applied, and exit\n  \
    --sensors   list temperature sensors with their current readings, and exit";

fn main() -> ExitCode {
    let mut check = false;
    let mut path = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--check" => check = true,
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

    run(config)
}

#[tokio::main(flavor = "current_thread")]
async fn run(config: &'static config::Config) -> ExitCode {
    notify::spawn(config);
    collect::spawn(config);
    jobs::spawn(&config.apps);
    match web::serve(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("web server on {}: {e}", config.listen);
            ExitCode::FAILURE
        }
    }
}
