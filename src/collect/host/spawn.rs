use crate::config::HostConfig;

use super::{cpu, memory, storage, temperature};

pub fn spawn(config: &'static HostConfig) {
    tokio::spawn(cpu::run(&config.cpu));
    tokio::spawn(memory::run(&config.memory));
    for mount in &config.storage {
        tokio::spawn(storage::run(mount));
    }
    temperature::spawn(&config.temperature);
}
