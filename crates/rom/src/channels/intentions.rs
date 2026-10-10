//! Freeze notification intentions after a committed resource action.
use crate::*;
use sha2::{Digest, Sha256};
impl Runtime {
    pub(crate) fn notification_intents(
        &self,
        row: &Row,
        effects: &[Intent],
        identity: &str,
        retry_epoch: u64,
        cause: Option<&Cause>,
    ) -> Result<Vec<PendingWork>> {
        let mut work = vec![];
        for (ordinal, intent) in effects.iter().enumerate() {
            let Some(version) = intent.delivery_version else {
                continue;
            };
            let def = self
                .0
                .channels
                .get(&intent.channel)
                .ok_or(Error::Unregistered)?;
            if def.version != version {
                return Err(Error::invalid("channel", "version"));
            }
            (def.validate)(&intent.payload)?;
            let cause = cause.cloned().unwrap_or_else(|| Cause {
                retry_epoch,
                root: identity.into(),
                parent: None,
                depth: 0,
                started_at: self.0.clock.now(),
                path: vec![],
            });
            let identity = json!([
                "rom-notification-v1",
                cause.root,
                cause.path,
                def.name,
                ordinal
            ])
            .to_string();
            let id = format!("{:x}", Sha256::digest(identity.as_bytes()));
            work.push(PendingWork {
                id,
                cause,
                definition: def.name.clone(),
                version,
                not_before: intent.not_before,
                delivery_profile: def.profile.clone(),
                service_key: def.actor.key(),
                payload: WorkPayload::Notification {
                    source: row.clone(),
                    payload: intent.payload.clone(),
                },
            });
        }
        Ok(work)
    }
}
