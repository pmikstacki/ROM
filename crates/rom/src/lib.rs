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
pub use rom_derive::Resource;
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

mod execution;
mod persistence;
mod query;
mod resource;
pub use execution::*;
pub use persistence::*;
pub use query::*;
pub use resource::*;
