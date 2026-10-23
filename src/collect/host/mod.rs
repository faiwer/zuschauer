//! Host metrics: CPU, memory and storage.

mod cpu;
#[cfg(target_os = "macos")]
mod mach;
mod memory;
mod storage;

use crate::config::HostConfig;

pub fn spawn(config: &'static HostConfig) {
    tokio::spawn(cpu::run(&config.cpu));
    tokio::spawn(memory::run(&config.memory));
}
