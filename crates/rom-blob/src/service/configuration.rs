//! Host composition and validation of blob providers and service limits.
use super::{
    BlobService, Limits,
    lifecycle::{Inner, Lifecycle},
};
use crate::{BlobStore, Error, Result};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{Semaphore, watch};
pub struct BlobServiceBuilder {
    runtime: rom::Runtime,
    stores: BTreeMap<String, Arc<dyn BlobStore>>,
    limits: Limits,
    duplicate: bool,
}
impl BlobServiceBuilder {
    pub fn store(mut self, name: &str, store: Arc<dyn BlobStore>) -> Self {
        self.duplicate |= self.stores.insert(name.into(), store).is_some();
        self
    }
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    pub fn build(self) -> Result<BlobService> {
        if self.duplicate {
            return Err(Error::Invalid);
        }
        BlobService::new(self.runtime, self.stores, self.limits)
    }
}
impl BlobService {
    /// Validated limits shared by the service and its transport adapters.
    pub fn limits(&self) -> Limits {
        self.0.limits
    }
    /// True only when this service shares the exact runtime lifecycle and identity gate.
    pub fn uses_runtime(&self, runtime: &rom::Runtime) -> bool {
        self.0.runtime.same_instance(runtime)
    }
    pub fn builder(runtime: rom::Runtime) -> BlobServiceBuilder {
        BlobServiceBuilder {
            runtime,
            stores: BTreeMap::new(),
            limits: Limits::default(),
            duplicate: false,
        }
    }
    pub fn new(
        runtime: rom::Runtime,
        stores: BTreeMap<String, Arc<dyn BlobStore>>,
        limits: Limits,
    ) -> Result<Self> {
        if stores.is_empty()
            || stores.len() > 32
            || stores.keys().any(|s| s.is_empty() || s.len() > 64)
            || limits.operations == 0
            || limits.operations > 64
            || limits.blob_bytes == 0
            || limits.blob_bytes > 16 * 1024 * 1024
            || limits.chunk_bytes == 0
            || limits.chunk_bytes > limits.blob_bytes
            || limits.chunks == 0
            || limits.chunks > 65_536
            || limits.staging_timeout.is_zero()
        {
            return Err(Error::Invalid);
        }
        Ok(Self(Arc::new(Inner {
            runtime,
            stores,
            limits,
            admission: Arc::new(Semaphore::new(limits.operations)),
            lifecycle: Arc::new(Mutex::new(Lifecycle::default())),
            changed: watch::channel(0).0,
        })))
    }
}
