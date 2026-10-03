//! Driver-neutral exclusion for one Runtime per backing store.
use crate::{Error, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Shared Runtime claim. Handles to the same backing store must share this value.
#[derive(Clone, Default)]
pub struct StorageOwnership(Arc<AtomicBool>);

/// Exclusive Runtime claim, released only when this guard drops.
pub struct StorageOwner(Arc<AtomicBool>);

impl StorageOwnership {
    pub fn acquire(&self) -> Result<StorageOwner> {
        self.0
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| Error::Conflict)?;
        Ok(StorageOwner(self.0.clone()))
    }
}

impl Drop for StorageOwner {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;
