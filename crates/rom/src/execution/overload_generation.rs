//! Required cfg(test)-only post-IO hook. No production counter implementation.
use super::Runtime;
use super::overload_fixture::BOUND;
use crate::{Error, Result};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Boundary {
    Observation,
    Actor,
}
pub(crate) struct Seen {
    pub boundary: Boundary,
    pub release: tokio::sync::oneshot::Sender<()>,
}
pub(crate) struct GenerationHook {
    pub boundary: Boundary,
    entered: tokio::sync::mpsc::Sender<Seen>,
}
impl GenerationHook {
    pub(super) fn new(boundary: Boundary) -> (Arc<Self>, tokio::sync::mpsc::Receiver<Seen>) {
        let (entered, received) = tokio::sync::mpsc::channel(8);
        (Arc::new(Self { boundary, entered }), received)
    }
}
/// Future lifecycle/authority cfg(test) call site must call this only after
/// completed IO and actor/open checks, immediately before comparing generation.
pub(crate) async fn before_generation_compare(runtime: &Runtime, boundary: Boundary) -> Result<()> {
    // Per-Runtime state exists only in cfg(test), and never holds a guard across await.
    let hook = {
        runtime
            .0
            .generation_test_hook
            .lock()
            .map_err(|_| Error::Panicked)?
            .clone()
    };
    if let Some(hook) = hook.filter(|hook| hook.boundary == boundary) {
        let (release, wait) = tokio::sync::oneshot::channel();
        hook.entered
            .try_send(Seen { boundary, release })
            .map_err(|_| Error::Storage)?;
        tokio::time::timeout(BOUND, wait)
            .await
            .map_err(|_| Error::Storage)?
            .map_err(|_| Error::Storage)?;
    }
    Ok(())
}
