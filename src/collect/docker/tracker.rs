//! Containers are keyed by name, so a recreated container (new ID, same name)
//! continues its history. A container seen once is never forgotten: when it
//! stops being reported, it's marked `Gone`.
//!
//! Two kinds of state, with different lifetimes:
//! - The store (by name): status and history. Never dropped.
//! - The tracker's caches (by ID): name lookups and CPU baselines. An ID dies
//!   with its container: recreating one yields a new ID and the old one never
//!   comes back. So these are dropped as soon as an ID isn't reported, or
//!   they'd grow with every recreate.

use std::collections::{HashMap, HashSet};
use std::io;
use std::time::Instant;

use serde::Deserialize;

use crate::model::{EntityId, MetricKind, Sample, Status, now};
use crate::store::store;

use super::api::Api;
use super::cpu::cpu_percent;
use super::reading::{Reading, read};

#[derive(Default)]
pub(super) struct Tracker {
    /// Container ID → name. Saves an API inspect call per tick on Linux, where
    /// cgroups give only IDs.
    names: HashMap<String, String>,
    /// Container ID → previous CPU counter. By ID, not name: a recreated
    /// container's counter restarts from zero, so it needs a fresh baseline.
    prev_cpu: HashMap<String, (u64, Instant)>,
    /// Names reported in the last snapshot, to tell which ones went missing.
    present: HashSet<String>,
}

impl Tracker {
    pub async fn tick(&mut self, api: Option<&Api>) {
        let snapshot = match read(api).await {
            Ok(snapshot) => snapshot,
            Err(e) => {
                eprintln!("docker: {e}");
                return;
            }
        };

        let mut present = HashSet::new();
        let mut ids = HashSet::new();
        for reading in snapshot.containers {
            let name = self.name_of(&reading, api).await;
            let cpu = self.cpu_of(&reading, snapshot.cores);
            record(&name, &reading, cpu);
            ids.insert(reading.id);
            present.insert(name);
        }

        self.mark_gone(&present);
        self.present = present;
        self.forget_all_but(&ids);
    }

    /// `None` for a container's first reading: there's nothing to diff yet.
    fn cpu_of(&mut self, reading: &Reading, cores: u32) -> Option<f32> {
        let cur = (reading.cpu_usage_usec, reading.at);
        let prev = self.prev_cpu.insert(reading.id.clone(), cur)?;
        cpu_percent(prev.0, prev.1, cur.0, cur.1, cores)
    }

    /// Marks containers reported last time but not now.
    fn mark_gone(&self, present: &HashSet<String>) {
        let since = now();
        let mut store = store();
        for name in self.present.difference(present) {
            store.set_status(EntityId::Container(name.clone()), Status::Gone { since });
        }
    }

    /// Drops cached names and CPU baselines of IDs that weren't reported. The
    /// containers themselves aren't dropped: their status and history stay in
    /// the store under their name.
    fn forget_all_but(&mut self, ids: &HashSet<String>) {
        self.prev_cpu.retain(|id, _| ids.contains(id));
        self.names.retain(|id, _| ids.contains(id));
    }

    /// Falls back to the short ID when the name can't be looked up; the
    /// lookup is retried on the next tick.
    async fn name_of(&mut self, reading: &Reading, api: Option<&Api>) -> String {
        if let Some(name) = &reading.name {
            self.names.insert(reading.id.clone(), name.clone());
            return name.clone();
        }
        if let Some(name) = self.names.get(&reading.id) {
            return name.clone();
        }
        let short_id = reading.id[..12.min(reading.id.len())].to_owned();
        let Some(api) = api else {
            return short_id;
        };
        match inspect_name(api, &reading.id).await {
            Ok(name) => {
                self.names.insert(reading.id.clone(), name.clone());
                name
            }
            Err(e) => {
                eprintln!("docker: name of {short_id}: {e}");
                short_id
            }
        }
    }
}

fn record(name: &str, reading: &Reading, cpu: Option<f32>) {
    let entity = EntityId::Container(name.to_owned());
    let status = if reading.paused {
        Status::Paused
    } else {
        Status::Running
    };
    let mut store = store();
    store.set_status(entity.clone(), status);
    if let Some(percent) = cpu {
        store.push(entity.clone(), MetricKind::Cpu, Sample::now(percent));
    }
    if let Some(bytes) = reading.memory {
        store.push(entity, MetricKind::MemoryBytes, Sample::now(bytes as f32));
    }
}

async fn inspect_name(api: &Api, id: &str) -> io::Result<String> {
    #[derive(Deserialize)]
    struct Inspect {
        #[serde(rename = "Name")]
        name: String,
    }
    let inspect: Inspect = api.get(&format!("/containers/{id}/json")).await?;
    Ok(inspect.name.trim_start_matches('/').to_owned())
}
