//! Shared runtime state. Ownership changes stay in lifecycle operations.
use super::Limits;
use crate::Error;
use crate::{ActorGate, Clock, ReactionLimits, Registered, Storage, channels, reactions};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, atomic::AtomicU64},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
#[derive(Default)]
pub(super) struct Lifecycle {
    pub(super) closed: bool,
    pub(super) active: usize,
    pub(super) terminal: Option<Error>,
}
pub(crate) struct Inner {
    pub(crate) storage: Arc<dyn Storage>,
    pub(crate) pool: Arc<rayon::ThreadPool>,
    pub(crate) registry: BTreeMap<String, Arc<dyn Registered>>,
    pub(crate) reactions: BTreeMap<String, Arc<reactions::RegisteredReaction>>,
    pub(crate) reaction_limits: ReactionLimits,
    pub(crate) channels: BTreeMap<String, Arc<channels::RegisteredChannel>>,
    pub(crate) delivery_timeout: std::time::Duration,
    pub(crate) reaction_worker: Arc<Semaphore>,
    pub(crate) gate: Mutex<()>,
    pub(super) denied: Mutex<BTreeSet<String>>,
    pub(crate) admission: Arc<Semaphore>,
    pub(super) io: Arc<Semaphore>,
    pub(crate) subscriptions: Arc<Semaphore>,
    pub(super) lifecycle: Arc<Mutex<Lifecycle>>,
    pub(crate) changes: watch::Sender<u64>,
    pub(super) drained: watch::Sender<u64>,
    pub(super) generation: AtomicU64,
    pub(crate) limits: Limits,
    pub(crate) clock: Arc<dyn Clock>,
    pub(super) actor_gate: Option<Arc<dyn ActorGate>>,
    pub(super) _storage_owner: crate::StorageOwner,
}
/// Tracked by the runtime, owned by actual blocking work, never a caller future.
pub(crate) struct Work {
    pub(super) lifecycle: Arc<Mutex<Lifecycle>>,
    pub(super) drained: watch::Sender<u64>,
    pub(super) _io: Option<OwnedSemaphorePermit>,
}
#[derive(Clone)]
pub struct Runtime(pub(crate) Arc<Inner>);
