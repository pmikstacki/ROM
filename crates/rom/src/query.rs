use super::*;
pub(crate) mod subscription;
use subscription::Subscription;
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
            runtime
                .select_rows(&a, R::KIND, &query.spec)?
                .into_iter()
                .map(|row| {
                    runtime.require_complete(&a, Some(&row), &row)?;
                    typed(row)
                })
                .collect()
        })
        .await
    }
    pub async fn live<R: Resource>(&self, actor: &Actor, query: Query<R>) -> Result<Live<R>> {
        let subscription = Subscription::begin(self, actor)?;
        self.query(actor, &query).await?;
        Ok(Live {
            runtime: self.clone(),
            actor: actor.clone(),
            query,
            subscription,
        })
    }
}
pub struct Live<R> {
    runtime: Runtime,
    actor: Actor,
    query: Query<R>,
    subscription: Subscription,
}
impl<R: Resource> Live<R> {
    pub async fn changed(&mut self) -> Result<Vec<Snapshot<R>>> {
        let generation = self
            .subscription
            .pending(&self.runtime, &self.actor)
            .await?;
        let rows = self.runtime.query(&self.actor, &self.query).await?;
        self.subscription.acknowledge(generation);
        Ok(rows)
    }
}
