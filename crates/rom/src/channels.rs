//! Typed application functions delivering durable notification intentions.
//!
//! ```
//! use rom::{Actor, Channel, Delivery, DeliveryOutcome, PrincipalKind, Runtime};
//! const NOTICE: Channel<String> = Channel::new("account-notices", 1);
//! async fn application_send(delivery: Delivery<String>) -> DeliveryOutcome {
//!     // Pass delivery.id to a receiver that supports deduplication.
//!     assert!(!delivery.id.is_empty());
//!     DeliveryOutcome::Accepted
//! }
//! let service = Actor::trusted("host", "notifier").with_kind(PrincipalKind::Service);
//! let _builder = Runtime::builder().channel(NOTICE, service, application_send);
//! let intent = NOTICE.intent("account changed".to_owned());
//! assert_eq!(intent.delivery_version, Some(1));
//! ```
use super::*;
use sha2::{Digest, Sha256};
use std::{future::Future, pin::Pin, time::Duration};
/// A stable named, versioned payload handle, usable inside ordinary Resource actions.
pub struct Channel<P> {
    pub(crate) name: &'static str,
    pub(crate) version: u32,
    marker: PhantomData<fn(P)>,
}
impl<P> Copy for Channel<P> {}
impl<P> Clone for Channel<P> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: Input> Channel<P> {
    pub const fn new(name: &'static str, version: u32) -> Self {
        Self {
            name,
            version,
            marker: PhantomData,
        }
    }
    pub fn intent(self, payload: P) -> Intent {
        Intent {
            channel: self.name.into(),
            payload: payload.encode(),
            delivery_version: Some(self.version),
        }
    }
}
/// The receiver must use this stable id to deduplicate external effects, when supported.
pub struct Delivery<P> {
    pub id: String,
    pub attempt: u32,
    pub payload: P,
}
/// Acceptance is an external acknowledgment, not proof of exactly-once delivery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryOutcome {
    Accepted,
    Retryable,
    Permanent,
    Unknown,
    TimedOut,
    Panicked,
}
type DeliveryFuture = Pin<Box<dyn Future<Output = DeliveryOutcome> + Send>>;
type ValidatePayload = Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>;
pub(crate) struct RegisteredChannel {
    pub name: String,
    pub version: u32,
    pub actor: Actor,
    pub validate: ValidatePayload,
    pub send: Arc<dyn Fn(Delivery<Value>) -> DeliveryFuture + Send + Sync>,
}
impl RegisteredChannel {
    pub fn new<P, F, Fut>(channel: Channel<P>, actor: Actor, send: F) -> Self
    where
        P: Input,
        F: Fn(Delivery<P>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = DeliveryOutcome> + Send + 'static,
    {
        Self {
            name: channel.name.into(),
            version: channel.version,
            actor,
            validate: Arc::new(|value| {
                let decoded = P::decode(value.clone())?;
                if decoded.encode() != *value {
                    return Err(Error::invalid("channel", "noncanonical payload"));
                }
                Ok(())
            }),
            send: Arc::new(move |delivery| match P::decode(delivery.payload) {
                Ok(payload) => Box::pin(send(Delivery {
                    id: delivery.id,
                    attempt: delivery.attempt,
                    payload,
                })),
                Err(_) => Box::pin(async { DeliveryOutcome::Permanent }),
            }),
        }
    }
}
impl Runtime {
    pub(crate) fn notification_intents(
        &self,
        row: &Row,
        effects: &[Intent],
        identity: &str,
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
pub(crate) fn default_timeout() -> Duration {
    Duration::from_secs(5)
}
impl Runtime {
    /// Run a finite batch of reaction and notification work under the shared supervisor.
    pub async fn process_work(&self, max_steps: usize) -> Result<usize> {
        self.process_reactions(max_steps).await
    }
    /// Start the one generic runtime-owned work loop. Shutdown drains accepted deliveries.
    pub fn start_work(&self) -> Result<ReactionWorker> {
        self.start_reactions()
    }
    pub(crate) fn process_notification(&self, claim: WorkClaim) -> Result<()> {
        let pending = &claim.work.pending;
        let Some(def) = self
            .0
            .channels
            .get(&pending.definition)
            .filter(|d| d.version == pending.version && d.actor.key() == pending.service_key)
        else {
            return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::DefinitionChanged));
        };
        if claim.resolution_only {
            return self.finish_claim(
                &claim,
                WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
            );
        }
        let WorkPayload::Notification { source, payload } = &pending.payload else {
            return Err(Error::Storage);
        };
        if (def.validate)(payload).is_err() {
            return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::Invalid));
        }
        let authority = (|| {
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            self.check_authority(&def.actor)?;
            if source.value.is_none() {
                return Err(Error::Denied);
            }
            let current = self.0.storage.load(&source.key)?;
            self.require_complete(&def.actor, current.as_ref(), source)?;
            self.0
                .storage
                .reaction_update(WorkUpdate::DeliveryStarted {
                    claim: claim.key(),
                    now: self.0.clock.now(),
                })?;
            Ok(())
        })();
        if let Err(error) = authority {
            return match error {
                Error::Denied => self.finish_claim(&claim, WorkOutcome::Stop(StopReason::Denied)),
                Error::Conflict => Ok(()),
                _ => Err(error),
            };
        }
        let send = def.send.clone();
        let delivery = Delivery {
            id: pending.id.clone(),
            attempt: claim.work.attempts,
            payload: payload.clone(),
        };
        let timeout = self.0.delivery_timeout;
        // This is a bounded spawn_blocking job, not a Tokio worker thread. The host's
        // runtime owns the async callback; retain supervision until abort/join completes.
        let outcome = tokio::runtime::Handle::current().block_on(async move {
            let mut task = tokio::spawn(async move { send(delivery).await });
            match tokio::time::timeout(timeout, &mut task).await {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(_)) => DeliveryOutcome::Panicked,
                Err(_) => {
                    task.abort();
                    let _ = task.await;
                    DeliveryOutcome::TimedOut
                }
            }
        });
        match self
            .0
            .storage
            .reaction_update(WorkUpdate::DeliveryFinished {
                claim: claim.key(),
                now: self.0.clock.now(),
                outcome,
            }) {
            Ok(_) | Err(Error::Conflict) => Ok(()),
            Err(error) => Err(error),
        }
    }
}
