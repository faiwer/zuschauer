//! Collectors. Each one runs as its own task on its own `every` timer and
//! writes samples into the store.
//!
//! - `spawn`: starts all configured collectors
//! - `host`: CPU, memory, storage
//! - `docker`: containers (Docker or Podman)
//! - `apps`: HTTP(S) health checks

mod apps;
mod docker;
mod host;
mod spawn;

pub use spawn::spawn;
