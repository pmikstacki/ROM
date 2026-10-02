use super::*;

/// Resume position within a persisted history generation and Resource kind.
/// Positions acknowledge inspected history, including facts the caller cannot read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalCursor {
    pub generation: String,
    pub kind: String,
    pub position: u64,
}

/// Durable storage fact. Adapters return these only to the trusted runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEvent {
    pub position: u64,
    pub identity: String,
    pub row: Row,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalPage {
    pub events: Vec<JournalEvent>,
    pub cursor: JournalCursor,
}

/// Authorized historical output. Storage identity and unprojected rows never cross this API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct JournalView {
    pub position: u64,
    pub view: ProjectedView,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct JournalBatch {
    pub events: Vec<JournalView>,
    pub cursor: JournalCursor,
}
impl Runtime {
    /// Explicitly establish a new checkpoint after accepting history loss.
    /// For state recovery obtain head, then snapshot, then subscribe from head;
    /// post-head overlap is intentional and must be reconciled by revision.
    pub async fn journal_head(&self, actor: &Actor, kind: &str) -> Result<JournalCursor> {
        let kind = kind.to_owned();
        self.observe(actor, move |runtime| {
            if !runtime.0.registry.contains_key(&kind) {
                return Err(Error::Unregistered);
            }
            if !runtime.0.storage.supports_journal() {
                return Err(Error::Unsupported("journal".into()));
            }
            let cursor = runtime.0.storage.journal_head(&kind)?;
            if cursor.kind != kind {
                return Err(Error::Storage);
            }
            Ok(cursor)
        })
        .await
    }

    /// A single bounded page; cursor advancement also acknowledges filtered facts.
    pub async fn journal(
        &self,
        actor: &Actor,
        kind: &str,
        after: Option<&JournalCursor>,
    ) -> Result<JournalBatch> {
        let kind = kind.to_owned();
        let after = after.cloned();
        let actor_owned = actor.clone();
        self.observe(actor, move |runtime| {
            if !runtime.0.registry.contains_key(&kind) {
                return Err(Error::Unregistered);
            }
            if !runtime.0.storage.supports_journal() {
                return Err(Error::Unsupported("journal".into()));
            }
            let page = runtime.0.storage.journal(
                &kind,
                after.as_ref(),
                runtime.0.limits.snapshot_rows,
                runtime.0.limits.snapshot_bytes,
            )?;
            if page.events.len() > runtime.0.limits.snapshot_rows {
                return Err(Error::TooLarge);
            }
            if page.cursor.kind != kind
                || after.as_ref().is_some_and(|a| {
                    a.generation != page.cursor.generation
                        || a.kind != kind
                        || a.position > page.cursor.position
                })
            {
                return Err(Error::HistoryGap);
            }
            let mut bytes = 0usize;
            let mut previous = after.as_ref().map_or(0, |a| a.position);
            let mut events = Vec::new();
            for event in page.events {
                bytes = bytes
                    .checked_add(
                        serde_json::to_vec(&event)
                            .map_err(|_| Error::Storage)?
                            .len(),
                    )
                    .ok_or(Error::TooLarge)?;
                if bytes > runtime.0.limits.snapshot_bytes {
                    return Err(Error::TooLarge);
                }
                if event.position <= previous
                    || event.position > page.cursor.position
                    || event.row.key.kind != kind
                {
                    return Err(Error::Storage);
                }
                previous = event.position;
                let current = runtime.0.storage.load(&event.row.key)?;
                // Missing current state cannot establish current authorization for history.
                if current.is_none() {
                    continue;
                }
                match runtime.project_outcome(&actor_owned, current.as_ref(), &event.row) {
                    Ok(view) => events.push(JournalView {
                        position: event.position,
                        view,
                    }),
                    Err(Error::Denied) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(JournalBatch {
                events,
                cursor: page.cursor,
            })
        })
        .await
    }
    /// Ordered facts with caller-owned checkpoints, separate from coalescing live state.
    pub async fn subscribe(
        &self,
        actor: &Actor,
        kind: &str,
        after: Option<JournalCursor>,
    ) -> Result<JournalSubscription> {
        self.ensure_open()?;
        self.check_actor(actor)?;
        let permit = self
            .0
            .subscriptions
            .clone()
            .try_acquire_owned()
            .map_err(|e| match e {
                tokio::sync::TryAcquireError::Closed => Error::Closed,
                _ => Error::Overloaded,
            })?;
        let changes = self.0.changes.subscribe();
        self.journal(actor, kind, after.as_ref()).await?;
        Ok(JournalSubscription {
            runtime: self.clone(),
            actor: actor.clone(),
            kind: kind.to_owned(),
            cursor: after,
            changes,
            initial: true,
            _permit: permit,
        })
    }
}
pub struct JournalSubscription {
    runtime: Runtime,
    actor: Actor,
    kind: String,
    cursor: Option<JournalCursor>,
    changes: watch::Receiver<u64>,
    initial: bool,
    _permit: OwnedSemaphorePermit,
}
impl JournalSubscription {
    /// Cancelling this future does not acknowledge a page. A successful return
    /// advances this handle only; durable consumer checkpoints remain the caller's job.
    pub async fn next(&mut self) -> Result<JournalBatch> {
        loop {
            self.runtime.check_actor(&self.actor)?;
            self.runtime.ensure_open()?;
            self.changes.borrow_and_update();
            let batch = self
                .runtime
                .journal(&self.actor, &self.kind, self.cursor.as_ref())
                .await?;
            if self.initial
                || self.cursor.as_ref() != Some(&batch.cursor)
                || !batch.events.is_empty()
            {
                self.initial = false;
                self.cursor = Some(batch.cursor.clone());
                return Ok(batch);
            }
            self.changes.changed().await.map_err(|_| Error::Closed)?;
        }
    }
}
