//! Docker runs in a VM on macOS (OrbStack, Docker Desktop), so its cgroups
//! aren't visible here. Uses the API instead: one list of running and paused
//! containers, then one one-shot stats call per container. Slower than
//! cgroups, good enough for development.

use std::io;
use std::time::Instant;

use serde::Deserialize;

use super::api::Api;
use super::reading::{Reading, Snapshot};

/// `filters={"status":["running","paused"]}`, URL-encoded.
const LIST: &str =
    "/containers/json?filters=%7B%22status%22%3A%5B%22running%22%2C%22paused%22%5D%7D";

#[derive(Deserialize)]
struct ListItem {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Names")]
    names: Vec<String>,
    #[serde(rename = "State")]
    state: String,
}

#[derive(Deserialize)]
struct Stats {
    cpu_stats: CpuStats,
    memory_stats: MemoryStats,
}

#[derive(Deserialize)]
struct CpuStats {
    cpu_usage: CpuUsage,
    /// Cores of the VM the containers run in.
    online_cpus: Option<u32>,
}

#[derive(Deserialize)]
struct CpuUsage {
    /// Nanoseconds.
    total_usage: u64,
}

#[derive(Deserialize)]
struct MemoryStats {
    usage: Option<u64>,
    #[serde(default)]
    stats: MemoryDetails,
}

#[derive(Default, Deserialize)]
struct MemoryDetails {
    inactive_file: Option<u64>,
}

pub async fn read(api: Option<&Api>) -> io::Result<Snapshot> {
    let api =
        api.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no Docker API socket found"))?;
    let list: Vec<ListItem> = api.get(LIST).await?;
    let mut cores = 1;
    let mut containers = Vec::with_capacity(list.len());
    for item in list {
        match read_container(api, item).await {
            Ok((reading, online_cpus)) => {
                cores = online_cpus.unwrap_or(cores);
                containers.push(reading);
            }
            Err(e) => eprintln!("docker: {e}"),
        }
    }
    Ok(Snapshot { cores, containers })
}

/// Also returns the VM's core count, which comes with the stats.
async fn read_container(api: &Api, item: ListItem) -> io::Result<(Reading, Option<u32>)> {
    let path = format!("/containers/{}/stats?stream=false&one-shot=true", item.id);
    let stats: Stats = api.get(&path).await?;
    let at = Instant::now();
    let memory = stats
        .memory_stats
        .usage
        .map(|usage| usage.saturating_sub(stats.memory_stats.stats.inactive_file.unwrap_or(0)));
    let reading = Reading {
        name: item
            .names
            .first()
            .map(|n| n.trim_start_matches('/').to_owned()),
        id: item.id,
        at,
        cpu_usage_usec: stats.cpu_stats.cpu_usage.total_usage / 1000,
        memory,
        paused: item.state == "paused",
    };
    Ok((reading, stats.cpu_stats.online_cpus))
}
