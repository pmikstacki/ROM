use super::*;
/// Whether intake accepts work or shutdown is draining/completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum IntakeState {
    Open,
    Draining,
    Stopped,
}
/// Payload-free host operations snapshot. Permit availability is advisory under
/// concurrent work; the intake/owned-work pair is sampled under the lifecycle lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimeStatus {
    pub intake: IntakeState,
    pub failed: bool,
    /// Tracked I/O batches and worker-loop lifetimes, not detached host tasks.
    pub owned_work: usize,
    pub available_action_permits: usize,
    pub available_io_permits: usize,
    pub available_subscription_permits: usize,
    pub registered_resources: usize,
    pub registered_reactions: usize,
    pub registered_channels: usize,
}
impl RuntimeStatus {
    /// Ready means configured intake is open. Capacity/backpressure is reported separately.
    pub fn is_ready(&self) -> bool {
        self.intake == IntakeState::Open && !self.failed
    }
}
/// Conservative host policy. Counts include work whose caller stopped waiting.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub actions: usize,
    pub io_jobs: usize,
    pub subscriptions: usize,
    pub snapshot_rows: usize,
    pub snapshot_bytes: usize,
    pub command_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            actions: 8,
            io_jobs: 8,
            subscriptions: 64,
            snapshot_rows: 1024,
            snapshot_bytes: 1024 * 1024,
            command_bytes: 16 * 1024,
        }
    }
}
#[derive(Default)]
pub struct Builder {
    registry: BTreeMap<String, Arc<dyn Registered>>,
    reactions: BTreeMap<String, Arc<reactions::RegisteredReaction>>,
    reaction_limits: ReactionLimits,
    channels: BTreeMap<String, Arc<channels::RegisteredChannel>>,
    delivery_timeout: Option<std::time::Duration>,
    error: Option<Error>,
    limits: Limits,
    pub(crate) clock: Option<Arc<dyn Clock>>,
    actor_gate: Option<Arc<dyn ActorGate>>,
}
impl Builder {
    pub fn channel<P, F, Fut>(mut self, channel: Channel<P>, actor: Actor, send: F) -> Self
    where
        P: Input,
        F: Fn(Delivery<P>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = DeliveryOutcome> + Send + 'static,
    {
        if channel.name.is_empty()
            || channel.version == 0
            || actor.principal_kind() != PrincipalKind::Service
        {
            self.error = Some(Error::invalid("channel", "name/version/service"));
        }
        let def = channels::RegisteredChannel::new(channel, actor, send);
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

    pub fn resource<R: Resource>(mut self, d: Definition<R>) -> Self {
        let desc = d.descriptor();
        let mut names = BTreeSet::new();
        for field in &desc.fields {
            if let Err(error) = validate_shape(&field.shape, 0, None) {
                self.error = Some(error);
            }
        }
        if desc.kind != R::KIND
            || desc.kind.is_empty()
            || desc.version != 1
            || desc
                .fields
                .iter()
                .any(|f| f.name.is_empty() || !names.insert(f.name.clone()))
        {
            self.error = Some(Error::invalid(R::KIND, "descriptor"));
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
        })))
    }
}
#[derive(Default)]
struct Lifecycle {
    closed: bool,
    active: usize,
    terminal: Option<Error>,
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
    denied: Mutex<BTreeSet<String>>,
    pub(crate) admission: Arc<Semaphore>,
    io: Arc<Semaphore>,
    pub(crate) subscriptions: Arc<Semaphore>,
    lifecycle: Arc<Mutex<Lifecycle>>,
    pub(crate) changes: watch::Sender<u64>,
    drained: watch::Sender<u64>,
    generation: AtomicU64,
    pub(crate) limits: Limits,
    pub(crate) clock: Arc<dyn Clock>,
    actor_gate: Option<Arc<dyn ActorGate>>,
}
/// Tracked by the runtime, owned by actual blocking work, never a caller future.
pub(crate) struct Work {
    lifecycle: Arc<Mutex<Lifecycle>>,
    drained: watch::Sender<u64>,
    _io: Option<OwnedSemaphorePermit>,
}
impl Drop for Work {
    fn drop(&mut self) {
        drop(self._io.take());
        let mut state = self.lifecycle.lock().unwrap();
        state.active -= 1;
        self.drained.send_modify(|v| *v = v.wrapping_add(1));
    }
}
#[derive(Clone)]
pub struct Runtime(pub(crate) Arc<Inner>);
pub(crate) fn acquire(pool: &Arc<Semaphore>) -> Result<OwnedSemaphorePermit> {
    pool.clone().try_acquire_owned().map_err(|e| match e {
        tokio::sync::TryAcquireError::Closed => Error::Closed,
        tokio::sync::TryAcquireError::NoPermits => Error::Overloaded,
    })
}
impl Runtime {
    pub(crate) fn track_worker(&self) -> Result<Work> {
        let mut state = self.0.lifecycle.lock().map_err(|_| Error::Panicked)?;
        if let Some(e) = &state.terminal {
            return Err(e.clone());
        }
        if state.closed {
            return Err(Error::Closed);
        }
        state.active += 1;
        Ok(Work {
            lifecycle: self.0.lifecycle.clone(),
            drained: self.0.drained.clone(),
            _io: None,
        })
    }
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
    /// Host-only operational state with no actor, Resource identity or payload data.
    /// Stopped is published only after tracked work releases its adapter-owning references.
    pub fn status(&self) -> Result<RuntimeStatus> {
        let state = self.0.lifecycle.lock().map_err(|_| Error::Panicked)?;
        let intake = if !state.closed {
            IntakeState::Open
        } else if state.active > 0 {
            IntakeState::Draining
        } else {
            IntakeState::Stopped
        };
        Ok(RuntimeStatus {
            intake,
            failed: state.terminal.is_some(),
            owned_work: state.active,
            available_action_permits: self.0.admission.available_permits(),
            available_io_permits: self.0.io.available_permits(),
            available_subscription_permits: self.0.subscriptions.available_permits(),
            registered_resources: self.0.registry.len(),
            registered_reactions: self.0.reactions.len(),
            registered_channels: self.0.channels.len(),
        })
    }
    pub fn available_capacity(&self) -> usize {
        self.0.admission.available_permits()
    }
    pub fn available_io_capacity(&self) -> usize {
        self.0.io.available_permits()
    }
    /// Cheap observer lifecycle, expiry and local-revocation check for transport
    /// keepalives. This does not evaluate the authoritative actor gate or Resource
    /// policies; only the ordinary observation APIs authorize data delivery.
    pub fn observation_status(&self, actor: &Actor) -> Result<()> {
        self.ensure_open()?;
        self.check_actor(actor)
    }
    pub(crate) fn check_actor(&self, actor: &Actor) -> Result<()> {
        let now = match catch_unwind(AssertUnwindSafe(|| self.0.clock.now())) {
            Ok(now) => now,
            Err(_) => {
                self.fail_terminal();
                return Err(Error::Panicked);
            }
        };
        if actor.valid_until().is_some_and(|end| now >= end) {
            return Err(Error::Denied);
        }
        if self
            .0
            .denied
            .lock()
            .map_err(|_| Error::Panicked)?
            .contains(&actor.key())
        {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
    pub(crate) fn ensure_open(&self) -> Result<()> {
        let state = self.0.lifecycle.lock().unwrap();
        if let Some(error) = &state.terminal {
            return Err(error.clone());
        }
        if state.closed {
            Err(Error::Closed)
        } else {
            Ok(())
        }
    }
    /// Only call from bounded I/O work, holding the commit gate when consistency
    /// with other managed Resource mutations matters.
    pub(crate) fn check_authority(&self, actor: &Actor) -> Result<()> {
        self.check_actor(actor)?;
        if let Some(gate) = &self.0.actor_gate {
            gate.check(
                actor,
                &mut policy::GateRead {
                    storage: self.0.storage.as_ref(),
                    reads: 8,
                    bytes: self.0.limits.command_bytes,
                },
            )?;
        }
        Ok(())
    }
    fn invalidate(&self) {
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        self.0.changes.send_modify(|v| *v = v.wrapping_add(1));
    }
    pub fn revoke(&self, actor: &Actor) {
        self.0.denied.lock().unwrap().insert(actor.key());
        self.invalidate();
    }
    /// Trusted host metadata inspection; a transport must not equate this with public discovery.
    pub fn descriptor<R: Resource>(&self, actor: &Actor) -> Result<Descriptor> {
        self.check_actor(actor)?;
        self.0
            .registry
            .get(R::KIND)
            .map(|d| d.descriptor())
            .ok_or(Error::Unregistered)
    }
    fn fail_terminal(&self) {
        let mut state = self.0.lifecycle.lock().unwrap();
        state.terminal = Some(Error::Panicked);
        state.closed = true;
        self.0.admission.close();
        self.0.io.close();
        self.0.subscriptions.close();
        drop(state);
        self.invalidate();
    }
    pub(crate) async fn io<T: Send + 'static, F: FnOnce(&Runtime) -> Result<T> + Send + 'static>(
        &self,
        f: F,
    ) -> Result<T> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.0.lifecycle.lock().unwrap();
            if let Some(error) = &state.terminal {
                return Err(error.clone());
            }
            if state.closed {
                return Err(Error::Closed);
            }
            let permit = acquire(&self.0.io)?;
            state.active += 1;
            let work = Work {
                lifecycle: self.0.lifecycle.clone(),
                drained: self.0.drained.clone(),
                _io: Some(permit),
            };
            let runtime = self.clone();
            tokio::task::spawn_blocking(move || {
                let result = match catch_unwind(AssertUnwindSafe(|| f(&runtime))) {
                    Ok(result) => result,
                    Err(_) => {
                        runtime.fail_terminal();
                        Err(Error::Panicked)
                    }
                };
                // Release the adapter-owning Runtime before publishing the drain condition.
                drop(runtime);
                drop(work);
                let _ = sender.send(result);
            });
        }
        receiver.await.map_err(|_| Error::Panicked)?
    }
    pub(crate) async fn observe<
        T: Send + 'static,
        F: Fn(&Runtime) -> Result<T> + Send + Sync + 'static,
    >(
        &self,
        actor: &Actor,
        f: F,
    ) -> Result<T> {
        self.check_actor(actor)?;
        let f = Arc::new(f);
        for _ in 0..8 {
            let a = actor.clone();
            let operation = f.clone();
            let (result, generation) = self
                .io(move |runtime| {
                    let _guard = runtime.0.gate.lock().map_err(|_| Error::Panicked)?;
                    runtime.check_authority(&a)?;
                    let result = operation(runtime)?;
                    runtime.check_authority(&a)?;
                    Ok((result, runtime.0.generation.load(Ordering::SeqCst)))
                })
                .await?;
            self.check_actor(actor)?;
            self.ensure_open()?;
            if generation == self.0.generation.load(Ordering::SeqCst) {
                return Ok(result);
            }
        }
        Err(Error::Overloaded)
    }
    /// Trusted host identity integration, not a transport endpoint. Resolves a
    /// candidate through bounded read-only I/O, then applies current actor checks.
    /// The callback may be retried after a concurrent managed Resource mutation.
    pub async fn establish_actor<F>(&self, resolve: F) -> Result<Actor>
    where
        F: Fn(&mut dyn AuthorizationRead) -> Result<Actor> + Send + Sync + 'static,
    {
        let resolve = Arc::new(resolve);
        for _ in 0..8 {
            let resolve = resolve.clone();
            let (actor, generation) = self
                .io(move |runtime| {
                    let _guard = runtime.0.gate.lock().map_err(|_| Error::Panicked)?;
                    let actor = resolve(&mut policy::GateRead {
                        storage: runtime.0.storage.as_ref(),
                        reads: 8,
                        bytes: runtime.0.limits.command_bytes,
                    })?;
                    runtime.check_authority(&actor)?;
                    Ok((actor, runtime.0.generation.load(Ordering::SeqCst)))
                })
                .await?;
            self.check_actor(&actor)?;
            self.ensure_open()?;
            if generation == self.0.generation.load(Ordering::SeqCst) {
                return Ok(actor);
            }
        }
        Err(Error::Overloaded)
    }
    pub async fn execute<R: Resource>(
        &self,
        actor: &Actor,
        cmd: Command<R>,
    ) -> Result<Snapshot<R>> {
        typed(self.invoke(actor, cmd.into()).await?)
    }
    pub async fn invoke(&self, actor: &Actor, invocation: Invocation) -> Result<Row> {
        let row = self.invoke_row(actor, invocation).await?;
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let current = runtime.0.storage.load(&row.key)?;
            runtime.require_complete(&a, current.as_ref(), &row)?;
            Ok(row.clone().public_outcome())
        })
        .await
    }
    pub(crate) async fn invoke_row(&self, actor: &Actor, invocation: Invocation) -> Result<Row> {
        self.check_actor(actor)?;
        invocation.check_size(actor, self.0.limits.command_bytes)?;
        if invocation.idempotency.is_empty() || invocation.id.is_empty() {
            return Err(Error::invalid(&invocation.kind, "identity"));
        }
        let identity = invocation.durable_identity(actor);
        let cmd = invocation.into_command();
        let permit = acquire(&self.0.admission)?;
        let a = actor.clone();
        let row = self
            .io(move |runtime| {
                let _permit = permit;
                runtime.run(&a, cmd, identity, None)
            })
            .await?;
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime
                .0
                .registry
                .get(&row.key.kind)
                .ok_or(Error::Unregistered)?;
            let current = runtime.0.storage.load(&row.key)?;
            runtime.disclose(&a, &(), def.as_ref(), current.as_ref(), &row)?;
            Ok(row.clone())
        })
        .await
    }
    pub(crate) fn run(
        &self,
        actor: &Actor,
        cmd: invocation::ErasedCommand,
        identity: String,
        causal: Option<(Cause, ClaimKey)>,
    ) -> Result<Row> {
        let mut causal = causal;
        if let Some((cause, claim)) = &mut causal {
            cause.parent = Some(claim.id.clone());
        }
        let def = self.0.registry.get(&cmd.kind).ok_or(Error::Unregistered)?;
        let key = Key {
            kind: cmd.kind.clone(),
            id: cmd.id.clone(),
        };
        let input = match &cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) => def.normalize(v.clone())?,
            Mutation::Patch(fields) => serde_json::to_value(normalize_patch(def.as_ref(), fields)?)
                .map_err(|_| Error::Storage)?,
            Mutation::Delete => Value::Null,
            Mutation::Action(_, v) => v.clone(),
        };
        let explicit_fields = !matches!(cmd.mutation, Mutation::Action(_, _) | Mutation::Patch(_));
        let patch_fields = match &cmd.mutation {
            Mutation::Patch(fields) => Some(fields.keys().cloned().collect::<Vec<_>>()),
            _ => None,
        };
        let fingerprint = json!([cmd.expected, input]).to_string();
        let prior = {
            let denied = self.0.gate.lock().unwrap();
            self.check_authority(actor)?;
            let prior = self.0.storage.load(&key)?;
            if let Some(receipt) = self.0.storage.receipt(&identity)? {
                self.disclose(actor, &denied, def.as_ref(), prior.as_ref(), &receipt.row)?;
                if receipt.fingerprint != fingerprint {
                    return Err(Error::IdentityMismatch);
                }
                return Ok(receipt.row);
            }
            let authorization = prior
                .as_ref()
                .and_then(|r| r.value.as_ref())
                .or(match &cmd.mutation {
                    Mutation::Create(v) => Some(v),
                    _ => None,
                })
                .ok_or(Error::Missing)?;
            if !def.allows(actor, Access::Write, authorization) {
                return Err(Error::Denied);
            }
            if prior.as_ref().map(|r| r.revision) != cmd.expected {
                return Err(Error::Conflict);
            }
            prior
        };
        self.check_actor(actor)?;
        // Native business functions compute proposals off Tokio. They must be pure w.r.t. external effects.
        let (new_value, effects) = match cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) => (Some(def.normalize(v)?), vec![]),
            Mutation::Patch(fields) => {
                let mut value = prior
                    .as_ref()
                    .and_then(|r| r.value.clone())
                    .ok_or(Error::Missing)?;
                let map = value.as_object_mut().ok_or(Error::Storage)?;
                for (name, update) in normalize_patch(def.as_ref(), &fields)? {
                    match update {
                        FieldUpdate::Set(v) => {
                            map.insert(name, v);
                        }
                        FieldUpdate::Remove => {
                            map.remove(&name);
                        }
                    }
                }
                (Some(def.normalize(value)?), vec![])
            }
            Mutation::Delete => (None, vec![]),
            Mutation::Action(name, input) => {
                let value = prior
                    .as_ref()
                    .and_then(|r| r.value.clone())
                    .ok_or(Error::Missing)?;
                let action = def.action(&name)?;
                let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                self.0.pool.spawn(move || {
                    let result = catch_unwind(AssertUnwindSafe(|| action(value, input)))
                        .unwrap_or(Err(Error::Panicked));
                    let _ = sender.send(result);
                });
                let (v, e) = receiver.recv().map_err(|_| Error::Panicked)??;
                (Some(def.normalize(v)?), e)
            }
        };
        if new_value
            .as_ref()
            .is_some_and(|v| !def.allows(actor, Access::Write, v))
        {
            return Err(Error::Denied);
        }
        if let Some(fields) = patch_fields.as_ref() {
            for field in fields {
                for value in [
                    prior.as_ref().and_then(|r| r.value.as_ref()),
                    new_value.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    if !def.allows_field(actor, Access::Write, field, value) {
                        return Err(Error::Denied);
                    }
                }
            }
        }
        self.authorize_fields(
            actor,
            def.as_ref(),
            prior.as_ref().and_then(|r| r.value.as_ref()),
            new_value.as_ref(),
            explicit_fields,
        )?;
        if effects.len() > 8
            || effects
                .iter()
                .map(|e| e.payload.to_string().len() + e.channel.len())
                .sum::<usize>()
                > self.0.limits.command_bytes
            || new_value
                .as_ref()
                .is_some_and(|v| v.to_string().len() > self.0.limits.command_bytes)
        {
            return Err(Error::TooLarge);
        }
        let denied = self.0.gate.lock().unwrap();
        self.check_authority(actor)?;
        let current = self.0.storage.load(&key)?;
        if let Some(receipt) = self.0.storage.receipt(&identity)? {
            self.disclose(actor, &denied, def.as_ref(), current.as_ref(), &receipt.row)?;
            if receipt.fingerprint != fingerprint {
                return Err(Error::IdentityMismatch);
            }
            return Ok(receipt.row);
        }
        // Recheck authoritative current state after CPU work and before conditional commit.
        if let Some(v) = current.as_ref().and_then(|r| r.value.as_ref())
            && !def.allows(actor, Access::Write, v)
        {
            return Err(Error::Denied);
        }
        if current.as_ref().map(|r| r.revision) != cmd.expected {
            return Err(Error::Conflict);
        }
        self.authorize_fields(
            actor,
            def.as_ref(),
            current.as_ref().and_then(|r| r.value.as_ref()),
            new_value.as_ref(),
            explicit_fields,
        )?;
        let changed = current.as_ref().and_then(|r| r.value.as_ref()) != new_value.as_ref();
        let revision = cmd
            .expected
            .unwrap_or(0)
            .checked_add(u64::from(changed))
            .ok_or(Error::TooLarge)?;
        let row = Row {
            key,
            revision,
            protected: ProtectedMetadata {
                deletion_authorization: if new_value.is_none() {
                    current.as_ref().and_then(|r| r.value.clone())
                } else {
                    None
                },
            },
            value: new_value,
        };
        // No-op effects are rejected: an external effect needs a committed transition in this slice.
        if !changed && !effects.is_empty() {
            return Err(Error::invalid(&cmd.kind, "no-op effects"));
        }
        let mut reactions = if changed {
            self.reaction_intents(
                current.as_ref(),
                &row,
                &identity,
                causal.as_ref().map(|(cause, _)| cause),
            )?
        } else {
            vec![]
        };
        reactions.extend(self.notification_intents(
            &row,
            &effects,
            &identity,
            causal.as_ref().map(|(cause, _)| cause),
        )?);
        let bundle = Bundle {
            reactions,
            reaction_limits: Some(self.0.reaction_limits.clone()),
            completed_work: causal.map(|(_, claim)| (claim, self.0.clock.now())),
            expected: cmd.expected,
            receipt: Receipt {
                identity,
                fingerprint,
                row: row.clone(),
            },
            changed,
            effects,
        };
        self.check_authority(actor)?;
        let receipt = self.0.storage.commit(&bundle);
        // Unknown may mean committed: invalidate even when the adapter loses its acknowledgment.
        if receipt.is_ok() || receipt == Err(Error::Unknown) {
            self.invalidate();
        }
        let receipt = receipt?;
        self.disclose(
            actor,
            &denied,
            def.as_ref(),
            Some(&receipt.row),
            &receipt.row,
        )?;
        Ok(receipt.row)
    }
    pub(crate) fn disclose(
        &self,
        actor: &Actor,
        _guard: &(),
        def: &dyn Registered,
        current: Option<&Row>,
        outcome: &Row,
    ) -> Result<()> {
        self.check_authority(actor)?;
        if current.is_some_and(|r| r.value.is_none()) && outcome.value.is_some() {
            return Err(Error::Denied);
        }
        // A deletion still discloses an identity and revision. Require both current
        // and historical row authorization; legacy tombstones without context deny.
        for row in current.into_iter().chain(std::iter::once(outcome)) {
            let value = row.authorization_value().ok_or(Error::Denied)?;
            if !def.allows(actor, Access::Read, value) {
                return Err(Error::Denied);
            }
        }
        Ok(())
    }
    pub(crate) fn authorize_read(
        &self,
        actor: &Actor,
        def: &dyn Registered,
        row: &Row,
    ) -> Result<()> {
        self.disclose(actor, &(), def, Some(row), row)?;
        self.require_complete(actor, Some(row), row)
    }
    fn authorize_fields(
        &self,
        actor: &Actor,
        def: &dyn Registered,
        prior: Option<&Value>,
        proposed: Option<&Value>,
        explicit: bool,
    ) -> Result<()> {
        for field in def.descriptor().fields {
            if !explicit
                && prior.and_then(|v| v.get(&field.name))
                    == proposed.and_then(|v| v.get(&field.name))
            {
                continue;
            }
            for value in [prior, proposed].into_iter().flatten() {
                if !def.allows_field(actor, Access::Write, &field.name, value) {
                    return Err(Error::Denied);
                }
            }
        }
        Ok(())
    }
    /// Every caller observes the same runtime-owned drain condition. Cancelling a waiter cannot lose work.
    pub async fn shutdown(&self) -> Result<()> {
        let mut changed = self.0.drained.subscribe();
        {
            let mut state = self.0.lifecycle.lock().unwrap();
            state.closed = true;
            self.0.admission.close();
            self.0.io.close();
            self.0.subscriptions.close();
        }
        self.invalidate();
        loop {
            {
                let state = self.0.lifecycle.lock().unwrap();
                if state.active == 0 {
                    return state.terminal.clone().map_or(Ok(()), Err);
                }
            }
            changed.changed().await.map_err(|_| Error::Panicked)?;
        }
    }
}
