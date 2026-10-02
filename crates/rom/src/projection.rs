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
        let def = self
            .0
            .registry
            .get(&outcome.key.kind)
            .ok_or(Error::Unregistered)?;
        if let Some(value) = outcome.value.as_ref() {
            for field in def.descriptor().fields {
                if !def.allows_field(actor, Access::Read, &field.name, value)
                    || current
                        .and_then(|r| r.value.as_ref())
                        .is_some_and(|v| !def.allows_field(actor, Access::Read, &field.name, v))
                {
                    return Err(Error::Denied);
                }
            }
        }
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
        self.query_spec_projected(actor, kind, QuerySpec::equal(field, value))
            .await
    }
    pub async fn query_spec_projected(
        &self,
        actor: &Actor,
        kind: &str,
        spec: QuerySpec,
    ) -> Result<Vec<ProjectedView>> {
        let kind = kind.to_owned();
        let a = actor.clone();
        self.observe(actor, move |runtime| {
            runtime
                .select_rows(&a, &kind, &spec)?
                .iter()
                .map(|row| runtime.project_outcome(&a, Some(row), row))
                .collect()
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
        self.live_spec_projected(actor, kind, QuerySpec::equal(field, value))
            .await
    }
    pub async fn live_spec_projected(
        &self,
        actor: &Actor,
        kind: &str,
        spec: QuerySpec,
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
        self.query_spec_projected(actor, kind, spec.clone()).await?;
        Ok(LiveProjected {
            runtime: self.clone(),
            actor: actor.clone(),
            kind: kind.into(),
            spec,
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
    spec: QuerySpec,
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
                .query_spec_projected(&self.actor, &self.kind, self.spec.clone())
                .await?;
            self.delivered_generation = Some(generation);
            return Ok(rows);
        }
    }
}
