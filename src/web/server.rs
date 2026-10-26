use std::io;

use axum::routing::{get, post};
use axum::{Router, middleware};
use tokio::net::TcpListener;

use crate::config::Config;

use super::api::{jobs, status};
use super::pages;
use super::timing::timing;

/// Runs until the process is killed.
pub async fn serve(config: &'static Config) -> io::Result<()> {
    let app = Router::new()
        .route("/", get(pages::index))
        .route("/favicon.svg", get(pages::favicon))
        .route("/entity/{kind}/{id}", get(pages::entity))
        .route("/status", get(status::status))
        .route("/jobs/{app}/{job}/{event}", post(jobs::report))
        .layer(middleware::from_fn(timing))
        .with_state(config);
    let listener = TcpListener::bind(config.listen).await?;
    eprintln!("listening on http://{}", config.listen);
    axum::serve(listener, app).await
}
