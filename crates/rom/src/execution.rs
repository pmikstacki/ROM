use super::*;
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
    error: Option<Error>,
    limits: Limits,
    clock: Option<Arc<dyn Clock>>,
}
impl Builder {
    pub fn resource<R: Resource>(mut self, d: Definition<R>) -> Self {
        let desc = d.descriptor();
        let mut names = BTreeSet::new();
        if let Some(field)=desc.fields.iter().find(|f|matches!(&f.shape,Shape::Nullable(inner) if matches!(inner.as_ref(),Shape::Nullable(_)))) {self.error=Some(Error::Unsupported(format!("{}: {}: nested nullable has no unambiguous codec",R::KIND,field.name)));}
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
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    pub fn build(self, storage: Arc<dyn Storage>, pool: Arc<rayon::ThreadPool>) -> Result<Runtime> {
        if let Some(error) = self.error {
            return Err(error);
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
            gate: Mutex::new(()),
            denied: Mutex::new(BTreeSet::new()),
            admission: Arc::new(Semaphore::new(l.actions)),
            io: Arc::new(Semaphore::new(l.io_jobs)),
            subscriptions: Arc::new(Semaphore::new(l.subscriptions)),
            lifecycle: Mutex::new(Lifecycle::default()),
            changes,
            drained,
            generation: AtomicU64::new(0),
            limits: l,
            clock: self.clock.unwrap_or_else(|| Arc::new(SystemClock)),
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
    pool: Arc<rayon::ThreadPool>,
    pub(crate) registry: BTreeMap<String, Arc<dyn Registered>>,
    gate: Mutex<()>,
    denied: Mutex<BTreeSet<String>>,
    admission: Arc<Semaphore>,
    io: Arc<Semaphore>,
    pub(crate) subscriptions: Arc<Semaphore>,
    lifecycle: Mutex<Lifecycle>,
    pub(crate) changes: watch::Sender<u64>,
    drained: watch::Sender<u64>,
    generation: AtomicU64,
    pub(crate) limits: Limits,
    clock: Arc<dyn Clock>,
}
/// Tracked by the runtime, owned by actual blocking work, never a caller future.
struct Work {
    runtime: Runtime,
    _io: Option<OwnedSemaphorePermit>,
}
impl Drop for Work {
    fn drop(&mut self) {
        drop(self._io.take());
        let mut state = self.runtime.0.lifecycle.lock().unwrap();
        state.active -= 1;
        self.runtime
            .0
            .drained
            .send_modify(|v| *v = v.wrapping_add(1));
    }
}
#[derive(Clone)]
pub struct Runtime(pub(crate) Arc<Inner>);
fn acquire(pool: &Arc<Semaphore>) -> Result<OwnedSemaphorePermit> {
    pool.clone().try_acquire_owned().map_err(|e| match e {
        tokio::sync::TryAcquireError::Closed => Error::Closed,
        tokio::sync::TryAcquireError::NoPermits => Error::Overloaded,
    })
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
    pub fn available_capacity(&self) -> usize {
        self.0.admission.available_permits()
    }
    pub fn available_io_capacity(&self) -> usize {
        self.0.io.available_permits()
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
    async fn io<T: Send + 'static, F: FnOnce(&Runtime) -> Result<T> + Send + 'static>(
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
                runtime: self.clone(),
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
                // Drop all work-owned permits and decrement active before notifying the waiter.
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
                    runtime.check_actor(&a)?;
                    let result = operation(runtime)?;
                    runtime.check_actor(&a)?;
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
    pub async fn execute<R: Resource>(
        &self,
        actor: &Actor,
        cmd: Command<R>,
    ) -> Result<Snapshot<R>> {
        self.check_actor(actor)?;
        if cmd.identity.is_empty() || cmd.id.is_empty() {
            return Err(Error::invalid(R::KIND, "identity"));
        }
        let size = match &cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) | Mutation::Action(_, v) => {
                v.to_string().len()
            }
            Mutation::Delete => 0,
        };
        if size
            .checked_add(cmd.id.len())
            .and_then(|n| n.checked_add(cmd.identity.len()))
            .is_none_or(|n| n > self.0.limits.command_bytes)
        {
            return Err(Error::TooLarge);
        }
        let permit = acquire(&self.0.admission)?;
        let a = actor.clone();
        let row = self
            .io(move |runtime| {
                let _permit = permit;
                runtime.run::<R>(&a, cmd)
            })
            .await?;
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
            let current = runtime.0.storage.load(&row.key)?;
            runtime.disclose(&a, &(), def.as_ref(), current.as_ref(), &row)?;
            typed(row.clone())
        })
        .await
    }
    fn run<R: Resource>(&self, actor: &Actor, cmd: Command<R>) -> Result<Row> {
        let def = self.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
        let key = Key {
            kind: R::KIND.into(),
            id: cmd.id.clone(),
        };
        let operation = match &cmd.mutation {
            Mutation::Create(_) => json!(["standard", "create"]),
            Mutation::Replace(_) => json!(["standard", "replace"]),
            Mutation::Delete => json!(["standard", "delete"]),
            Mutation::Action(n, _) => json!(["custom", n]),
        };
        let identity = json!([
            actor.authority,
            actor.subject,
            R::KIND,
            cmd.id,
            operation,
            cmd.identity
        ])
        .to_string();
        let input = match &cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) => def.normalize(v.clone())?,
            Mutation::Delete => Value::Null,
            Mutation::Action(_, v) => v.clone(),
        };
        let fingerprint = json!([cmd.expected, input]).to_string();
        let prior = {
            let denied = self.0.gate.lock().unwrap();
            self.check_actor(actor)?;
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
        self.check_actor(actor)?;
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
        let changed = current.as_ref().and_then(|r| r.value.as_ref()) != new_value.as_ref();
        let revision = cmd
            .expected
            .unwrap_or(0)
            .checked_add(u64::from(changed))
            .ok_or(Error::TooLarge)?;
        let row = Row {
            key,
            revision,
            value: new_value,
        };
        // No-op effects are rejected: an external effect needs a committed transition in this slice.
        if !changed && !effects.is_empty() {
            return Err(Error::invalid(R::KIND, "no-op effects"));
        }
        let bundle = Bundle {
            expected: cmd.expected,
            receipt: Receipt {
                identity,
                fingerprint,
                row: row.clone(),
            },
            changed,
            effects,
        };
        self.check_actor(actor)?;
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
    fn disclose(
        &self,
        actor: &Actor,
        _guard: &(),
        def: &dyn Registered,
        current: Option<&Row>,
        outcome: &Row,
    ) -> Result<()> {
        self.check_actor(actor)?;
        if current.is_some_and(|r| r.value.is_none()) && outcome.value.is_some() {
            return Err(Error::Denied);
        }
        // Tombstone replay returns revision only and uses host revocation. Historical field projection omitted.
        if let Some(v) = current.and_then(|r| r.value.as_ref())
            && !def.allows(actor, Access::Read, v)
        {
            return Err(Error::Denied);
        }
        if let Some(v) = &outcome.value
            && !def.allows(actor, Access::Read, v)
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub(crate) fn authorize_read(
        &self,
        actor: &Actor,
        def: &dyn Registered,
        row: &Row,
    ) -> Result<()> {
        self.disclose(actor, &(), def, Some(row), row)
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
