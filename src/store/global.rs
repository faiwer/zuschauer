use std::sync::{Mutex, MutexGuard, PoisonError};

use super::history::Store;

static STORE: Mutex<Store> = Mutex::new(Store::new());

/// Locks the global store. Never hold the guard across an `.await`.
pub fn store() -> MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(PoisonError::into_inner)
}
