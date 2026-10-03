//! Typed, post-commit reactions. Mapping functions must be pure and bounded.
use super::*;
mod receipt;
/// A custom action input for an existing target Resource.
pub struct Target<I> {
    pub id: String,
    pub input: I,
}
impl<I> Target<I> {
    pub fn new(id: impl Into<String>, input: I) -> Self {
        Self {
            id: id.into(),
            input,
        }
    }
}
type Mapper = Arc<dyn Fn(Row) -> Result<Vec<(String, Value)>> + Send + Sync>;
pub(crate) struct RegisteredReaction {
    pub name: String,
    pub version: u32,
    pub source: String,
    pub target: String,
    pub action: String,
    pub actor: Actor,
    pub dependencies: BTreeSet<String>,
    pub mapper: Mapper,
}
pub struct Reaction<S, T, I> {
    inner: RegisteredReaction,
    marker: PhantomData<fn(S, T, I)>,
}
impl<S: Resource, T: Resource, I: Input> Reaction<S, T, I> {
    pub fn new(
        name: &str,
        version: u32,
        actor: Actor,
        action: Action<T, I>,
        map: fn(&Snapshot<S>) -> Result<Vec<Target<I>>>,
    ) -> Self {
        Self {
            inner: RegisteredReaction {
                name: name.into(),
                version,
                source: S::KIND.into(),
                target: T::KIND.into(),
                action: action.name.into(),
                actor,
                dependencies: BTreeSet::new(),
                mapper: Arc::new(move |row| {
                    map(&typed::<S>(row)?).map(|targets| {
                        targets
                            .into_iter()
                            .map(|t| (t.id, I::encode(&t.input)))
                            .collect()
                    })
                }),
            },
            marker: PhantomData,
        }
    }
    /// Omit dependencies for conservative routing on every semantic source change.
    pub fn depends_on<F: Field>(mut self, field: FieldRef<S, F>) -> Self {
        self.inner.dependencies.insert(field.name.into());
        self
    }
    pub(crate) fn erase(self) -> RegisteredReaction {
        self.inner
    }
}
pub struct ReactionWorker {
    task: tokio::task::JoinHandle<Result<()>>,
}
impl ReactionWorker {
    pub async fn join(self) -> Result<()> {
        self.task.await.map_err(|_| Error::Panicked)?
    }
}
impl Runtime {
    /// Start one generic runtime worker. Runtime shutdown stops it; dropping this observer does not.
    pub fn start_reactions(&self) -> Result<ReactionWorker> {
        self.ensure_open()?;
        let permit = execution::acquire(&self.0.reaction_worker)?;
        let lifetime = self.track_worker()?;
        let runtime = self.clone();
        let mut changes = self.0.changes.subscribe();
        let task = tokio::spawn(async move {
            let _permit = permit;
            let result=async {loop {
                if runtime.ensure_open() == Err(Error::Closed) {
                    return Ok(());
                }
                runtime.ensure_open()?;
                match runtime
                    .process_reactions((runtime.0.reaction_limits.max_work as usize).min(32))
                    .await
                {
                    Ok(0) | Err(Error::Overloaded) => {}
                    Ok(_) => continue,
                    Err(Error::Closed) => return Ok(()),
                    Err(e) => return Err(e),
                }
                tokio::select! {_ = changes.changed()=>{},_ = tokio::time::sleep(std::time::Duration::from_millis(100))=>{}}
            }}.await;
            drop(runtime);
            drop(_permit);
            drop(lifetime);
            result
        });
        Ok(ReactionWorker { task })
    }
    /// Process a finite batch on the runtime's supervised storage executor.
    /// Accepted work continues if its caller is cancelled. No per-Resource worker is needed.
    pub async fn process_reactions(&self, max_steps: usize) -> Result<usize> {
        if max_steps == 0 || max_steps > self.0.reaction_limits.max_work as usize {
            return Err(Error::TooLarge);
        }
        if !self.0.storage.supports_reactions() {
            return Err(Error::Unsupported("durable reactions".into()));
        }
        let permit = execution::acquire(&self.0.admission)?;
        self.io(move |runtime| {
            let _permit = permit;
            let mut completed = 0;
            for _ in 0..max_steps {
                runtime.ensure_open()?;
                let now = runtime.0.clock.now();
                let WorkResult::Claimed(claim) = runtime
                    .0
                    .storage
                    .reaction_update(WorkUpdate::Claim { now })?
                else {
                    break;
                };
                runtime.process_claim(*claim)?;
                completed += 1;
            }
            Ok(completed)
        })
        .await
    }
    fn process_claim(&self, claim: WorkClaim) -> Result<()> {
        if matches!(claim.work.pending.payload, WorkPayload::Notification { .. }) {
            return self.process_notification(claim);
        }
        let pending = &claim.work.pending;
        let Some(def) = self
            .0
            .reactions
            .get(&pending.definition)
            .filter(|d| d.version == pending.version && d.actor.key() == pending.service_key)
        else {
            return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::DefinitionChanged));
        };
        let result = (|| {
            self.check_authority(&def.actor)?;
            if let WorkPayload::Action(value) = &pending.payload {
                let invocation: Invocation =
                    serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
                invocation.check_size(&def.actor, self.0.limits.command_bytes)?;
                let identity = invocation.durable_identity(&def.actor);
                if invocation.retry_epoch != pending.cause.retry_epoch {
                    return Err(Error::Storage);
                }
                let replay_exists = {
                    let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
                    self.resolve_frozen_action(&def.actor, &invocation)?
                };
                if replay_exists {
                    return self.finish_claim(&claim, WorkOutcome::Done);
                }
                if claim.resolution_only {
                    return self.finish_claim(
                        &claim,
                        WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                    );
                }
                self.run(
                    &def.actor,
                    invocation.into_command(),
                    identity,
                    Some((pending.cause.clone(), claim.key())),
                )?;
                // The native target commit also marks this claim Done. Replayed receipts take the path above.
                return Ok(());
            }
            if claim.resolution_only {
                return self.finish_claim(
                    &claim,
                    WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                );
            }
            let WorkPayload::Source(source) = &pending.payload else {
                unreachable!()
            };
            if source.value.is_none() {
                return Err(Error::Denied);
            }
            {
                let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
                self.check_authority(&def.actor)?;
                let current = self.0.storage.load(&source.key)?;
                self.require_complete(&def.actor, current.as_ref(), source)?;
            }
            let mapper = def.mapper.clone();
            let source = source.clone();
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            self.0.pool.spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| mapper(source)))
                    .unwrap_or(Err(Error::Panicked));
                let _ = send.send(result);
            });
            let targets = receive.recv().map_err(|_| Error::Panicked)??;
            if targets.len() > self.0.reaction_limits.max_fanout {
                return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::Fanout));
            }
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            self.check_authority(&def.actor)?;
            let WorkPayload::Source(source) = &pending.payload else {
                unreachable!()
            };
            let current = self.0.storage.load(&source.key)?;
            self.require_complete(&def.actor, current.as_ref(), source)?;
            let mut children = vec![];
            for (index, (id, input)) in targets.into_iter().enumerate() {
                if id.is_empty() {
                    return Err(Error::invalid(&def.target, "reaction target"));
                }
                let target = self
                    .0
                    .storage
                    .load(&Key {
                        kind: def.target.clone(),
                        id: id.clone(),
                    })?
                    .ok_or(Error::Missing)?;
                let mut cause = pending.cause.clone();
                cause.parent = Some(pending.id.clone());
                cause.path.push(index.to_string());
                let work_id = json!([cause.root, cause.path]).to_string();
                let invocation = Invocation {
                    retry_epoch: cause.retry_epoch,
                    kind: def.target.clone(),
                    id,
                    expected: Some(target.revision),
                    idempotency: work_id.clone(),
                    operation: Operation::Action {
                        name: def.action.clone(),
                        input,
                    },
                };
                invocation.check_size(&def.actor, self.0.limits.command_bytes)?;
                children.push(PendingWork {
                    id: work_id,
                    cause,
                    definition: def.name.clone(),
                    version: def.version,
                    delivery_profile: DeliveryProfile::AtLeastOnce,
                    service_key: def.actor.key(),
                    payload: WorkPayload::Action(
                        serde_json::to_value(invocation).map_err(|_| Error::Storage)?,
                    ),
                });
            }
            self.0.storage.reaction_update(WorkUpdate::Materialize {
                claim: claim.key(),
                now: self.0.clock.now(),
                children,
            })?;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(Error::Unknown) => Ok(()),
            Err(e) => {
                let reason = match e {
                    Error::Denied => Some(StopReason::Denied),
                    Error::Conflict => Some(StopReason::Conflict),
                    Error::Missing => Some(StopReason::Missing),
                    Error::Invalid { .. }
                    | Error::TooLarge
                    | Error::IdentityMismatch
                    | Error::IdentityExpired => Some(StopReason::Invalid),
                    Error::Unregistered | Error::Unsupported(_) => {
                        Some(StopReason::DefinitionChanged)
                    }
                    Error::Panicked => Some(StopReason::CallbackPanicked),
                    _ => None,
                };
                self.finish_claim(&claim, reason.map_or(WorkOutcome::Retry, WorkOutcome::Stop))
            }
        }
    }
    pub(crate) fn finish_claim(&self, claim: &WorkClaim, outcome: WorkOutcome) -> Result<()> {
        match self.0.storage.reaction_update(WorkUpdate::Finish {
            claim: claim.key(),
            now: self.0.clock.now(),
            outcome,
        }) {
            Ok(_) | Err(Error::Conflict) => Ok(()),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn reaction_intents(
        &self,
        previous: Option<&Row>,
        row: &Row,
        identity: &str,
        retry_epoch: u64,
        cause: Option<&Cause>,
    ) -> Result<Vec<PendingWork>> {
        let mut work = vec![];
        for def in self
            .0
            .reactions
            .values()
            .filter(|d| d.source == row.key.kind)
        {
            if !def.dependencies.is_empty()
                && def.dependencies.iter().all(|field| {
                    previous
                        .and_then(|r| r.value.as_ref())
                        .and_then(|v| v.get(field))
                        == row.value.as_ref().and_then(|v| v.get(field))
                })
            {
                continue;
            }
            let mut cause = cause.cloned().unwrap_or_else(|| Cause {
                retry_epoch,
                root: identity.into(),
                parent: None,
                depth: 0,
                started_at: self.0.clock.now(),
                path: vec![],
            });
            cause.depth = cause.depth.checked_add(1).ok_or(Error::TooLarge)?;
            cause.path.push(def.name.clone());
            let id = json!([cause.root, cause.path]).to_string();
            work.push(PendingWork {
                id,
                cause,
                definition: def.name.clone(),
                version: def.version,
                delivery_profile: DeliveryProfile::AtLeastOnce,
                service_key: def.actor.key(),
                payload: WorkPayload::Source(row.clone()),
            });
        }
        if work.len() > self.0.reaction_limits.max_fanout {
            return Err(Error::TooLarge);
        }
        Ok(work)
    }
}
