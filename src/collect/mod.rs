//! Collectors. Each one runs as its own task on its own `every` timer and
//! writes samples into the store.
//!
//! - `spawn`: starts all configured collectors
//! - `host`: CPU, memory, storage, temperature
//! - `docker`: containers (Docker or Podman)
//! - `apps`: HTTP(S) health checks and certificates

mod apps;
mod docker;
mod host;
mod spawn;

pub use host::list_sensors;
pub use spawn::spawn;
