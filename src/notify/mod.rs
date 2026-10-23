//! Notifiers: deliver alert changes to the configured services.
//!
//! - `message`: what's sent, independent of the service
//! - `outbox`: a queue per notifier, delivery with retries
//! - `ntfy`: publishing to an ntfy server

mod message;
mod ntfy;
mod outbox;

pub use message::Message;
pub use outbox::{send, spawn};
