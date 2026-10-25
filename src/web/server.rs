use std::io;

use axum::Router;
use axum::routing::{get, post};
use tokio::net::TcpListener;

use crate::config::Config;

use super::api::jobs;
use super::pages;

/// Runs until the process is killed.
pub async fn serve(config: &'static Config) -> io::Result<()> {
    let app = Router::new()
        .route("/", get(pages::index))
        .route("/favicon.svg", get(pages::favicon))
        .route("/entity/{kind}/{id}", get(pages::entity))
        .route("/jobs/{app}/{job}/{event}", post(jobs::report))
        .with_state(config);
    let listener = TcpListener::bind(config.listen).await?;
    eprintln!("listening on http://{}", config.listen);
    axum::serve(listener, app).await
}
