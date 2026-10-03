//! Resource authoring types and internal runtime adapters.
mod command;
mod definition;
mod fields;
mod query;
mod schema;

pub use command::{Command, Snapshot};
pub use definition::{Action, Definition, Intent};
pub use fields::{Field, FiniteF64, Input, ResourceRef};
pub use query::{FieldRef, Query};
pub use schema::{Descriptor, FieldDescriptor, Resource, Shape};

pub(crate) use command::{Mutation, typed};
pub(crate) use definition::Registered;
pub(crate) use schema::{canonical_fields, matches_shape, validate_shape};
