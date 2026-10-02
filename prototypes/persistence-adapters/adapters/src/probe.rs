//! Explicitly test-only hooks, absent from default builds and core contracts.
use rom_persistence_core::Error;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checkpoint {
    AfterResource,
    AfterReceipt,
    AfterEvent,
    BeforeCommit,
    AfterCommit,
}
type Callback = dyn Fn(Checkpoint) -> Result<(), Error> + Send + Sync;
#[derive(Clone, Default)]
pub struct Probe(Option<Arc<Callback>>);
impl Probe {
    pub fn new(callback: impl Fn(Checkpoint) -> Result<(), Error> + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(callback)))
    }
    pub(crate) fn hit(&self, point: Checkpoint) -> Result<(), Error> {
        if let Some(callback) = &self.0 {
            callback(point)?;
        }
        Ok(())
    }
}
