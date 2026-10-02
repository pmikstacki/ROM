use super::*;
impl Runtime {
    pub async fn read<R: Resource>(&self, actor: &Actor, id: &str) -> Result<Snapshot<R>> {
        let key = Key {
            kind: R::KIND.into(),
            id: id.into(),
        };
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
            let row = runtime.0.storage.load(&key)?.ok_or(Error::Missing)?;
            if row.value.is_none() {
                return Err(Error::Denied);
            }
            runtime.authorize_read(&a, def.as_ref(), &row)?;
            typed(row)
        })
        .await
    }
    pub async fn query<R: Resource>(
        &self,
        actor: &Actor,
        query: &Query<R>,
    ) -> Result<Vec<Snapshot<R>>> {
        let query = query.clone();
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
            let descriptor = def.descriptor();
            let field = descriptor
                .fields
                .iter()
                .find(|f| f.name == query.field)
                .ok_or_else(|| Error::invalid(R::KIND, &query.field))?;
            if !matches_shape(&query.value, &field.shape) {
                return Err(Error::invalid(R::KIND, &query.field));
            }
            let rows = runtime.0.storage.snapshot(
                R::KIND,
                runtime.0.limits.snapshot_rows,
                runtime.0.limits.snapshot_bytes,
            )?;
            // Enforce the contract defensively for native adapters, as well as inside each provider.
            let mut bytes = 0usize;
            if rows.len() > runtime.0.limits.snapshot_rows {
                return Err(Error::TooLarge);
            }
            for row in &rows {
                bytes = bytes
                    .checked_add(serde_json::to_vec(row).map_err(|_| Error::Storage)?.len())
                    .ok_or(Error::TooLarge)?;
                if bytes > runtime.0.limits.snapshot_bytes {
                    return Err(Error::TooLarge);
                }
            }
            rows.into_iter()
                .filter(|r| {
                    r.value.as_ref().is_some_and(|v| {
                        def.allows(&a, Access::Read, v) && v.get(&query.field) == Some(&query.value)
                    })
                })
                .map(typed)
                .collect()
        })
        .await
    }
    pub async fn live<R: Resource>(&self, actor: &Actor, query: Query<R>) -> Result<Live<R>> {
        self.ensure_open()?;
        self.check_actor(actor)?;
        let permit = self
            .0
            .subscriptions
            .clone()
            .try_acquire_owned()
            .map_err(|e| match e {
                tokio::sync::TryAcquireError::Closed => Error::Closed,
                tokio::sync::TryAcquireError::NoPermits => Error::Overloaded,
            })?;
        let changes = self.0.changes.subscribe();
        self.query(actor, &query).await?;
        Ok(Live {
            runtime: self.clone(),
            actor: actor.clone(),
            query,
            changes,
            delivered_generation: None,
            _permit: permit,
        })
    }
}
pub struct Live<R> {
    runtime: Runtime,
    actor: Actor,
    query: Query<R>,
    changes: watch::Receiver<u64>,
    delivered_generation: Option<u64>,
    _permit: OwnedSemaphorePermit,
}
impl<R: Resource> Live<R> {
    pub async fn changed(&mut self) -> Result<Vec<Snapshot<R>>> {
        loop {
            self.runtime.check_actor(&self.actor)?;
            self.runtime.ensure_open()?;
            // watch tracks notifications; this handle separately tracks successful disclosure.
            // Clearing watch's marker must not acknowledge a failed or cancelled query.
            let generation = *self.changes.borrow_and_update();
            if self.delivered_generation == Some(generation) {
                self.changes.changed().await.map_err(|_| Error::Closed)?;
                continue;
            }
            let rows = self.runtime.query(&self.actor, &self.query).await?;
            // Capture before the query, so changes during it remain pending even if the
            // returned snapshot happened to include them. A redundant refresh is safe.
            self.delivered_generation = Some(generation);
            return Ok(rows);
        }
    }
}
