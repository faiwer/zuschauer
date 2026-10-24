//! The global in-memory history: entities keyed by `EntityId`, each holding
//! one `Series` per metric.
//!
//! - `global`: the process-wide store and `store()` to lock it
//! - `history`: `Store`, the map of entities
//! - `entity`: one entity's series and status
//! - `series`: a metric's points; `ring`: the fixed-size buffer behind it

mod entity;
mod global;
mod history;
mod ring;
mod series;

pub use entity::Entity;
pub use global::store;
pub use history::Store;
