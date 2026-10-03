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
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::{OwnedSemaphorePermit, watch};

mod error;
pub use error::{Error, Result};

mod discovery;
mod execution;
mod invocation;
mod journal;
mod persistence;
mod query;
mod references;
mod replay;
mod retry_epoch;
pub use retry_epoch::RetryEpochs;
mod resource;
pub use discovery::*;
pub use execution::*;
pub use invocation::*;
pub use journal::*;
pub use persistence::*;
pub use query::*;
pub use references::*;
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
mod storage_ownership;
pub use storage_ownership::{StorageOwner, StorageOwnership};

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
#[path = "input_codec.rs"]
pub mod __private;

mod query_storage;
pub use query_storage::{
    KindAdmission, QUERY_ENCODING_VERSION, QUERY_PROFILE_VERSION, QUERY_SEMANTICS_VERSION,
    QueryBounds, QueryCost, QueryEstimates, QueryRead, QuerySnapshot, QueryStrategy, ReadBinding,
    SelectionMode, StorageQuery, select_query_strategy,
};
