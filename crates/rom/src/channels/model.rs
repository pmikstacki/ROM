//! Typed channel handles and delivery outcomes.
use crate::*;
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
