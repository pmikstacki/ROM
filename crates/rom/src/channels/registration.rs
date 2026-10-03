//! Typed host registration of delivery profiles and trusted verification.
use crate::*;
use std::{future::Future, pin::Pin, time::Duration};
type DeliveryFuture = Pin<Box<dyn Future<Output = DeliveryOutcome> + Send>>;
type ValidatePayload = Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>;
pub(crate) struct RegisteredChannel {
    pub profile: DeliveryProfile,
    pub verifier: Option<Verifier>,
    pub verification_timeout: Option<Duration>,
    pub name: String,
    pub version: u32,
    pub actor: Actor,
    pub validate: ValidatePayload,
    pub send: Arc<dyn Fn(Delivery<Value>) -> DeliveryFuture + Send + Sync>,
}
impl RegisteredChannel {
    pub fn new<P, F, Fut>(registration: ChannelRegistration<P>, actor: Actor, send: F) -> Self
    where
        P: Input,
        F: Fn(Delivery<P>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = DeliveryOutcome> + Send + 'static,
    {
        let channel = registration.channel;
        Self {
            profile: registration.profile,
            verifier: registration.verifier,
            verification_timeout: registration.timeout,
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
pub(crate) fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// Stable delivery identity and bounded optional provider lookup reference.
/// This is supplied by Runtime to a trusted host callback, not decoded from wire input.
pub struct DeliveryReconciliation {
    pub id: String,
    pub evidence_ref: Option<String>,
}
/// Trusted provider verification. NotAccepted assures the old attempt cannot succeed later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeliveryVerification {
    Accepted { evidence: String },
    NotAccepted { evidence: String },
    Unresolved,
}
type VerificationFuture = Pin<Box<dyn Future<Output = DeliveryVerification> + Send>>;
pub(crate) type Verifier = Arc<dyn Fn(DeliveryReconciliation) -> VerificationFuture + Send + Sync>;
/// One typed channel declaration, including its persisted delivery contract.
pub struct ChannelRegistration<P> {
    pub(crate) channel: Channel<P>,
    pub(crate) profile: DeliveryProfile,
    pub(crate) verifier: Option<Verifier>,
    pub(crate) timeout: Option<Duration>,
}
impl<P: Input> Channel<P> {
    pub fn delivery_profile(self, profile: DeliveryProfile) -> ChannelRegistration<P> {
        ChannelRegistration {
            channel: self,
            profile,
            verifier: None,
            timeout: None,
        }
    }
}
impl<P: Input> ChannelRegistration<P> {
    /// Callbacks must yield and avoid blocking executor threads. Timeout aborts and joins
    /// the async task; it cannot preempt arbitrary blocking Rust code.
    pub fn verifier<F, Fut>(mut self, verifier: F) -> Self
    where
        F: Fn(DeliveryReconciliation) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = DeliveryVerification> + Send + 'static,
    {
        self.verifier = Some(Arc::new(move |input| Box::pin(verifier(input))));
        self
    }
    /// Omission uses the Runtime delivery timeout. Explicit deadlines must be positive and finite.
    pub fn verification_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl RegisteredChannel {
    pub(crate) fn matches(&self, pending: &PendingWork) -> bool {
        self.name == pending.definition
            && self.version == pending.version
            && self.actor.key() == pending.service_key
            && self.profile == pending.delivery_profile
    }
    pub(crate) fn validate_timeout(&self, default: Duration) -> Result<()> {
        let timeout = self.verification_timeout.unwrap_or(default);
        if timeout.is_zero() || std::time::Instant::now().checked_add(timeout).is_none() {
            return Err(Error::invalid(
                "channel",
                "verification timeout must be positive and timer-representable",
            ));
        }
        Ok(())
    }
}
