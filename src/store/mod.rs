//! The global in-memory history: `static STORE: Mutex<Store>`, entities keyed
//! by `EntityId`, each holding one `Series` per metric.

mod ring;
mod series;
