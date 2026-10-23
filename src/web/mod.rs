//! HTTP server: routes, server-rendered pages and SVG sparklines.

mod pages;
mod svg;

use std::io;
use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;

/// Runs until the process is killed.
pub async fn serve(addr: SocketAddr) -> io::Result<()> {
    let app = Router::new().route("/", get(pages::index));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("listening on http://{addr}");
    axum::serve(listener, app).await
}
