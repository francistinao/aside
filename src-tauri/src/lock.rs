//! Mutex access that survives a poisoned lock. Nothing we guard has an invariant a panic
//! elsewhere could break, so the inner value is always safe to keep using.

use std::sync::{Mutex, MutexGuard};

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
