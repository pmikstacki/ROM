use super::*;

/// Authorized partial view. Deliberately cannot be decoded as a complete Resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectedView {
    pub key: Key,
    pub revision: u64,
    pub value: Option<Map<String, Value>>,
}

impl Runtime {
    /// Must run inside bounded I/O and the runtime commit gate.
    pub(crate) fn project_outcome(
        &self,
        actor: &Actor,
        current: Option<&Row>,
        outcome: &Row,
    ) -> Result<ProjectedView> {
        let def = self
            .0
            .registry
            .get(&outcome.key.kind)
            .ok_or(Error::Unregistered)?;
        self.disclose(actor, &(), def.as_ref(), current, outcome)?;
        let value = outcome.value.as_ref().map(|value| {
            let mut fields = Map::new();
            for field in def.descriptor().fields {
                let current_allowed = current
                    .and_then(|r| r.value.as_ref())
                    .is_none_or(|v| def.allows_field(actor, Access::Read, &field.name, v));
                if current_allowed
                    && def.allows_field(actor, Access::Read, &field.name, value)
                    && let Some(v) = value.get(&field.name)
                {
                    fields.insert(field.name, v.clone());
                }
            }
            fields
        });
        Ok(ProjectedView {
            key: outcome.key.clone(),
            revision: outcome.revision,
            value,
        })
    }
    pub(crate) fn require_complete(
        &self,
        actor: &Actor,
        current: Option<&Row>,
        outcome: &Row,
    ) -> Result<()> {
        let projected = self.project_outcome(actor, current, outcome)?;
        if projected.value.as_ref().map(|m| m.len())
            != outcome
                .value
                .as_ref()
                .and_then(|v| v.as_object().map(|m| m.len()))
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub async fn invoke_projected(
        &self,
        actor: &Actor,
        invocation: Invocation,
    ) -> Result<ProjectedView> {
        let row = self.invoke_row(actor, invocation).await?;
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let current = runtime.0.storage.load(&row.key)?;
            runtime.project_outcome(&a, current.as_ref(), &row)
        })
        .await
    }
    pub async fn read_projected(
        &self,
        actor: &Actor,
        kind: &str,
        id: &str,
    ) -> Result<ProjectedView> {
        let key = Key {
            kind: kind.into(),
            id: id.into(),
        };
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let row = runtime.0.storage.load(&key)?.ok_or(Error::Missing)?;
            if row.value.is_none() {
                return Err(Error::Denied);
            }
            runtime.project_outcome(&a, Some(&row), &row)
        })
        .await
    }
    pub async fn query_projected(
        &self,
        actor: &Actor,
        kind: &str,
        field: &str,
        value: Value,
    ) -> Result<Vec<ProjectedView>> {
        let kind = kind.to_owned();
        let field = field.to_owned();
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            let def = runtime.0.registry.get(&kind).ok_or(Error::Unregistered)?;
            let descriptor = def.descriptor();
            let shape = &descriptor
                .fields
                .iter()
                .find(|f| f.name == field)
                .ok_or_else(|| Error::invalid(&kind, &field))?
                .shape;
            if !matches_shape(&value, shape) {
                return Err(Error::invalid(&kind, &field));
            }
            if !def.allows_query(&a, &field) {
                return Err(Error::Denied);
            }
            let rows = runtime.0.storage.snapshot(
                &kind,
                runtime.0.limits.snapshot_rows,
                runtime.0.limits.snapshot_bytes,
            )?;
            if rows.len() > runtime.0.limits.snapshot_rows {
                return Err(Error::TooLarge);
            }
            let mut bytes = 0usize;
            let mut out = vec![];
            for row in rows {
                bytes = bytes
                    .checked_add(serde_json::to_vec(&row).map_err(|_| Error::Storage)?.len())
                    .ok_or(Error::TooLarge)?;
                if bytes > runtime.0.limits.snapshot_bytes {
                    return Err(Error::TooLarge);
                }
                if row.value.as_ref().is_some_and(|v| {
                    def.allows(&a, Access::Read, v) && v.get(&field) == Some(&value)
                }) {
                    out.push(runtime.project_outcome(&a, Some(&row), &row)?);
                }
            }
            Ok(out)
        })
        .await
    }
    pub async fn live_projected(
        &self,
        actor: &Actor,
        kind: &str,
        field: &str,
        value: Value,
    ) -> Result<LiveProjected> {
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
        self.query_projected(actor, kind, field, value.clone())
            .await?;
        Ok(LiveProjected {
            runtime: self.clone(),
            actor: actor.clone(),
            kind: kind.into(),
            field: field.into(),
            value,
            changes,
            delivered_generation: None,
            _permit: permit,
        })
    }
}

pub struct LiveProjected {
    runtime: Runtime,
    actor: Actor,
    kind: String,
    field: String,
    value: Value,
    changes: watch::Receiver<u64>,
    delivered_generation: Option<u64>,
    _permit: OwnedSemaphorePermit,
}
impl LiveProjected {
    /// Pending work is acknowledged only after a successful authorized query.
    pub async fn changed(&mut self) -> Result<Vec<ProjectedView>> {
        loop {
            self.runtime.check_actor(&self.actor)?;
            self.runtime.ensure_open()?;
            let generation = *self.changes.borrow_and_update();
            if self.delivered_generation == Some(generation) {
                self.changes.changed().await.map_err(|_| Error::Closed)?;
                continue;
            }
            let rows = self
                .runtime
                .query_projected(&self.actor, &self.kind, &self.field, self.value.clone())
                .await?;
            self.delivered_generation = Some(generation);
            return Ok(rows);
        }
    }
}
