//! Durable reaction intents from semantic Resource changes.
use crate::*;
impl Runtime {
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
                not_before: None,
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
