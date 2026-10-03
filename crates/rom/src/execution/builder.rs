//! Runtime composition and registration before intake opens.
use super::{Inner, Limits, Runtime, state::Lifecycle};
use crate::{
    Actor, ActorGate, Channel, Clock, Definition, Delivery, DeliveryOutcome, Error, Input,
    PrincipalKind, Reaction, ReactionLimits, Registered, Resource, Result, RetryEpochs, Storage,
    SystemClock, channels, reactions, validate_shape,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, atomic::AtomicU64},
};
use tokio::sync::{Semaphore, watch};
#[derive(Default)]
pub struct Builder {
    retry_fence: RetryEpochs,
    registry: BTreeMap<String, Arc<dyn Registered>>,
    reactions: BTreeMap<String, Arc<reactions::RegisteredReaction>>,
    reaction_limits: ReactionLimits,
    channels: BTreeMap<String, Arc<channels::RegisteredChannel>>,
    delivery_timeout: Option<std::time::Duration>,
    error: Option<Error>,
    limits: Limits,
    pub(crate) clock: Option<Arc<dyn Clock>>,
    actor_gate: Option<Arc<dyn ActorGate>>,
    operator_authorizer: Option<Arc<dyn crate::OperatorAuthorizer>>,
    operator_limits: crate::OperatorLimits,
}
impl Builder {
    /// Minimum persisted boundaries from trusted state outside rollback backups.
    pub fn retry_fence(mut self, fence: RetryEpochs) -> Self {
        self.retry_fence = fence;
        self
    }
    pub fn channel<P, F, Fut>(self, channel: Channel<P>, actor: Actor, send: F) -> Self
    where
        P: Input,
        F: Fn(Delivery<P>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = DeliveryOutcome> + Send + 'static,
    {
        self.channel_with(
            channel.delivery_profile(crate::DeliveryProfile::AtLeastOnce),
            actor,
            send,
        )
    }
    pub fn channel_with<P, F, Fut>(
        mut self,
        registration: crate::ChannelRegistration<P>,
        actor: Actor,
        send: F,
    ) -> Self
    where
        P: Input,
        F: Fn(Delivery<P>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = DeliveryOutcome> + Send + 'static,
    {
        let channel = registration.channel;
        if channel.name.is_empty()
            || channel.version == 0
            || actor.principal_kind() != PrincipalKind::Service
        {
            self.error = Some(Error::invalid("channel", "name/version/service"));
        }
        let def = channels::RegisteredChannel::new(registration, actor, send);
        if self
            .channels
            .insert(def.name.clone(), Arc::new(def))
            .is_some()
        {
            self.error = Some(Error::Duplicate("channel".into()));
        }
        self
    }
    pub fn delivery_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.delivery_timeout = Some(timeout);
        self
    }

    pub fn reaction<S: Resource, T: Resource, I: Input>(
        mut self,
        reaction: Reaction<S, T, I>,
    ) -> Self {
        let d = reaction.erase();
        if d.name.is_empty() || d.version == 0 || d.actor.principal_kind() != PrincipalKind::Service
        {
            self.error = Some(Error::invalid("reaction", "name/version/service"));
        }
        if self.reactions.insert(d.name.clone(), Arc::new(d)).is_some() {
            self.error = Some(Error::Duplicate("reaction".into()));
        }
        self
    }
    pub fn reaction_limits(mut self, limits: ReactionLimits) -> Self {
        self.reaction_limits = limits;
        self
    }

    pub fn resource<R: Resource>(mut self, mut d: Definition<R>) -> Self {
        let desc = d.descriptor();
        let mut names = BTreeSet::new();
        for field in &desc.fields {
            if let Err(error) = validate_shape(&field.shape, 0, None) {
                self.error = Some(error);
            }
        }
        if desc.kind != R::KIND
            || desc.kind.is_empty()
            || desc.version == 0
            || desc
                .fields
                .iter()
                .any(|f| f.name.is_empty() || !names.insert(f.name.clone()))
        {
            self.error = Some(Error::invalid(R::KIND, "descriptor"));
        }
        if let Some(error) = d.replay_error.take() {
            self.error = Some(error);
        }
        if d.duplicate || self.registry.insert(R::KIND.into(), Arc::new(d)).is_some() {
            self.error = Some(Error::Duplicate(R::KIND.into()));
        }
        self
    }
    pub fn capacity(mut self, n: usize) -> Self {
        self.limits.actions = n;
        self
    }
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = Some(clock);
        self
    }
    pub fn actor_gate(mut self, gate: Arc<dyn ActorGate>) -> Self {
        self.actor_gate = Some(gate);
        self
    }
    pub fn operator_authorizer(mut self, authorizer: Arc<dyn crate::OperatorAuthorizer>) -> Self {
        self.operator_authorizer = Some(authorizer);
        self
    }
    pub fn operator_limits(mut self, limits: crate::OperatorLimits) -> Self {
        self.operator_limits = limits;
        self
    }
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    pub fn build(self, storage: Arc<dyn Storage>, pool: Arc<rayon::ThreadPool>) -> Result<Runtime> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let kinds: BTreeSet<String> = self.registry.keys().cloned().collect();
        for definition in self.registry.values() {
            for field in &definition.descriptor().fields {
                validate_shape(&field.shape, 0, Some(&kinds))?;
            }
        }
        self.reaction_limits.validate()?;
        self.operator_limits.validate()?;
        let delivery_timeout = self
            .delivery_timeout
            .unwrap_or_else(channels::default_timeout);
        if !self.channels.is_empty()
            && (delivery_timeout.is_zero()
                || delivery_timeout
                    >= std::time::Duration::from_secs(self.reaction_limits.lease_seconds))
        {
            return Err(Error::invalid(
                "channel",
                "timeout must be positive and shorter than lease",
            ));
        }
        for channel in self.channels.values() {
            channel.validate_timeout(delivery_timeout)?;
        }
        if (!self.reactions.is_empty() || !self.channels.is_empty())
            && !storage.supports_reactions()
        {
            return Err(Error::Unsupported("durable reactions".into()));
        }
        for d in self.reactions.values() {
            let source = self.registry.get(&d.source).ok_or(Error::Unregistered)?;
            self.registry
                .get(&d.target)
                .ok_or(Error::Unregistered)?
                .action(&d.action)?;
            if d.dependencies
                .iter()
                .any(|name| !source.descriptor().fields.iter().any(|f| &f.name == name))
            {
                return Err(Error::invalid(&d.source, "reaction dependency"));
            }
        }
        let c = storage.capabilities();
        if !c.atomic_bundle || !c.snapshots || !c.effects {
            return Err(Error::Unsupported(
                "atomic state/event/receipt/effect bundle + authoritative snapshots".into(),
            ));
        }
        let l = self.limits;
        if [
            l.actions,
            l.io_jobs,
            l.subscriptions,
            l.snapshot_rows,
            l.snapshot_bytes,
            l.command_bytes,
        ]
        .contains(&0)
        {
            return Err(Error::Unsupported("limits must be nonzero".into()));
        }
        if [l.actions, l.io_jobs, l.subscriptions]
            .iter()
            .any(|n| *n > Semaphore::MAX_PERMITS)
        {
            return Err(Error::Unsupported(
                "concurrency exceeds semaphore limit".into(),
            ));
        }
        let storage_owner = storage.acquire_owner()?;
        storage.retry_epochs()?.check_fence(self.retry_fence)?;
        storage.register(
            &self
                .registry
                .values()
                .map(|d| d.descriptor())
                .collect::<Vec<_>>(),
        )?;
        let (changes, _) = watch::channel(0);
        let (drained, _) = watch::channel(0);
        Ok(Runtime(Arc::new(Inner {
            storage,
            pool,
            registry: self.registry,
            reactions: self.reactions,
            reaction_limits: self.reaction_limits,
            channels: self.channels,
            delivery_timeout,
            operator_authorizer: self.operator_authorizer,
            operator_limits: self.operator_limits,
            reaction_worker: Arc::new(Semaphore::new(1)),
            gate: Mutex::new(()),
            denied: Mutex::new(BTreeSet::new()),
            admission: Arc::new(Semaphore::new(l.actions)),
            io: Arc::new(Semaphore::new(l.io_jobs)),
            subscriptions: Arc::new(Semaphore::new(l.subscriptions)),
            lifecycle: Arc::new(Mutex::new(Lifecycle::default())),
            changes,
            drained,
            generation: AtomicU64::new(0),
            limits: l,
            clock: self.clock.unwrap_or_else(|| Arc::new(SystemClock)),
            actor_gate: self.actor_gate,
            _storage_owner: storage_owner,
        })))
    }
}
impl Runtime {
    pub fn builder() -> Builder {
        Builder::default()
    }
    pub fn shared_cpu_pool(threads: usize) -> Result<Arc<rayon::ThreadPool>> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map(Arc::new)
            .map_err(|_| Error::Storage)
    }
}
