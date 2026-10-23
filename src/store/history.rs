use std::collections::BTreeMap;

use crate::model::{EntityId, MetricKind, Sample, Status, Timestamp};

use super::entity::Entity;
use super::series::Series;

#[derive(Debug)]
pub struct Store {
    entities: BTreeMap<EntityId, Entity>,
}

impl Store {
    pub(super) const fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, entity: EntityId, metric: MetricKind, sample: Sample) {
        self.entities
            .entry(entity)
            .or_default()
            .push(metric, sample);
    }

    pub fn set_status(&mut self, entity: EntityId, status: Status) {
        self.entities.entry(entity).or_default().set_status(status);
    }

    pub fn set_error(&mut self, entity: EntityId, error: Option<String>) {
        self.entities.entry(entity).or_default().set_error(error);
    }

    pub fn set_expires_at(&mut self, entity: EntityId, at: Timestamp) {
        self.entities.entry(entity).or_default().set_expires_at(at);
    }

    pub fn entity(&self, entity: &EntityId) -> Option<&Entity> {
        self.entities.get(entity)
    }

    pub fn series(&self, entity: &EntityId, metric: MetricKind) -> Option<&Series> {
        self.entities.get(entity)?.series(metric)
    }

    /// Sorted by `EntityId`.
    pub fn entities(&self) -> impl Iterator<Item = (&EntityId, &Entity)> {
        self.entities.iter()
    }
}
