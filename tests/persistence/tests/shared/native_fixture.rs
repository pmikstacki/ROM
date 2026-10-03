//! Local owners adapt native test inspection to the public conformance seam.
use super::{Arc, Backend, Error, Inspect, Result, Scratch, Storage};
pub(super) fn facts(store: &dyn Inspect) -> rom_conformance::StorageFacts {
    rom_conformance::StorageFacts {
        counts: store.counts(),
        events: store.events(),
        effects: store.effects(),
    }
}
struct NativeFixture {
    scratch: Scratch,
    backend: Backend,
    store: Option<Arc<dyn Inspect>>,
}
impl rom_conformance::StorageFixture for NativeFixture {
    fn storage(&self) -> Arc<dyn Storage> {
        self.store.as_ref().unwrap().clone()
    }
    fn facts(&self) -> Result<rom_conformance::StorageFacts> {
        Ok(facts(&**self.store.as_ref().unwrap()))
    }
    fn reopen(&mut self) -> Result<()> {
        if Arc::strong_count(self.store.as_ref().unwrap()) != 1 {
            return Err(Error::Storage);
        }
        drop(self.store.take());
        self.store = Some(self.backend.open(&self.scratch.path()));
        Ok(())
    }
}
impl Drop for NativeFixture {
    fn drop(&mut self) {
        drop(self.store.take());
    }
}
pub(super) fn fixture(backend: Backend) -> Result<Box<dyn rom_conformance::StorageFixture>> {
    let scratch = Scratch::new();
    let store = Some(backend.open(&scratch.path()));
    Ok(Box::new(NativeFixture {
        scratch,
        backend,
        store,
    }))
}
