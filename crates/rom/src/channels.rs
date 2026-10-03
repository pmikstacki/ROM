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
mod execution;
mod intentions;
mod model;
mod registration;
pub(crate) mod supervision;
pub use model::*;
pub use registration::{ChannelRegistration, DeliveryReconciliation, DeliveryVerification};
pub(crate) use registration::{RegisteredChannel, default_timeout};
