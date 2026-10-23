//! Host metrics: CPU, memory and storage.

mod cpu;
mod memory;
mod storage;

use crate::config::HostConfig;

pub fn spawn(config: &'static HostConfig) {
    tokio::spawn(cpu::run(&config.cpu));
}
