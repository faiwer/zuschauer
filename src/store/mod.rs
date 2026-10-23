//! The global in-memory history: entities keyed by `EntityId`, each holding
//! one `Series` per metric.

mod ring;
mod series;

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::model::{EntityId, MetricKind, Sample};

pub use series::Series;

static STORE: Mutex<Store> = Mutex::new(Store::new());

/// Locks the global store. Never hold the guard across an `.await`.
pub fn store() -> MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Debug)]
pub struct Store {
    entities: BTreeMap<EntityId, Entity>,
}

#[derive(Debug, Default)]
struct Entity {
    series: BTreeMap<MetricKind, Series>,
}

impl Store {
    const fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, entity: EntityId, metric: MetricKind, sample: Sample) {
        self.entities
            .entry(entity)
            .or_default()
            .series
            .entry(metric)
            .or_default()
            .push(sample);
    }

    pub fn series(&self, entity: &EntityId, metric: MetricKind) -> Option<&Series> {
        self.entities.get(entity)?.series.get(&metric)
    }
}
