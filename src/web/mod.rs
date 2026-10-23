//! HTTP server: routes, server-rendered pages and SVG sparklines.

mod pages;
mod svg;

use std::io;

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;

use crate::config::Config;

/// Runs until the process is killed.
pub async fn serve(config: &'static Config) -> io::Result<()> {
    let app = Router::new()
        .route("/", get(pages::index))
        .with_state(config);
    let listener = TcpListener::bind(config.listen).await?;
    eprintln!("listening on http://{}", config.listen);
    axum::serve(listener, app).await
}
