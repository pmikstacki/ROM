//! Typed Resources with shared execution, persistence and live observations.
//!
//! ```
//! use rom::{Resource, Runtime};
//! #[derive(Clone, Resource)]
//! #[resource(name = "tasks")]
//! struct Task { title: String, done: bool }
//! let definition = Task::definition().policy(|actor, _, _| actor.subject == "owner");
//! let _builder = Runtime::builder().resource(definition);
//! assert!(Task::decode(rom::json!({"title": "one", "done": "invalid"})).is_err());
//! ```
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#[cfg(feature = "derive")]
pub use rom_derive::{Input, Resource};
use serde::{Deserialize, Serialize};
pub use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot, watch};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid { kind: String, field: String },
    Unsupported(String),
    Duplicate(String),
    Unregistered,
    Denied,
    Conflict,
    IdentityMismatch,
    Missing,
    Overloaded,
    Closed,
    NotCommitted,
    Unknown,
    Panicked,
    Storage,
    TooLarge,
    HistoryGap,
}
impl Error {
    pub fn invalid(kind: &str, field: &str) -> Self {
        Self::Invalid {
            kind: kind.into(),
            field: field.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

mod discovery;
mod execution;
mod invocation;
mod journal;
mod persistence;
mod query;
mod resource;
pub use discovery::*;
pub use execution::*;
pub use invocation::*;
pub use journal::*;
pub use persistence::*;
pub use query::*;
pub use resource::*;

mod policy;
pub use policy::*;
mod source;
pub use source::*;
mod projection;
pub use projection::*;

mod reaction_work;
pub use reaction_work::*;
mod storage_state;
pub use storage_state::*;

mod query_eval;
mod query_spec;
pub use query_spec::*;
mod reactions;
pub use reactions::*;

mod patch;
pub use patch::*;
mod channels;
pub use channels::*;

/// Implementation details for ROM-generated codecs, not an application extension contract.
#[doc(hidden)]
pub mod __private {
    pub fn decode_input_member<T: crate::Field>(value: Option<crate::Value>) -> crate::Result<T> {
        crate::resource::validate_shape(&T::shape(), 0, None)?;
        match value {
            Some(value) => T::decode(value),
            None => T::decode_missing(),
        }
    }
}
