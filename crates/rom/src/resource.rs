//! Resource authoring types and internal runtime adapters.
mod command;
mod definition;
mod enum_labels;
mod fields;
mod input_descriptor;
mod presentation;
mod query;
mod schema;

pub use command::{Command, Snapshot};
pub use definition::{Action, Definition, Intent};
pub use enum_labels::FieldEnumLabels;
pub(crate) use fields::wrapper_path;
pub use fields::{Field, FiniteF64, Input, ResourceRef};
pub(crate) use input_descriptor::visible_shape;
pub use input_descriptor::{
    CodecIdentity, CodecWrapper, FieldCodec, InputDescriptor, InputFieldDescriptor,
};
pub use presentation::{
    FieldPresentation, PresentationGroup, ResourcePresentation, SettingsPresentation,
};
pub use query::{FieldRef, Query};
pub use schema::{Descriptor, FieldDescriptor, Resource, Shape};

pub(crate) use command::{Mutation, typed};
pub(crate) use definition::Registered;
pub(crate) use schema::{canonical_fields, matches_shape, validate_shape};

#[cfg(test)]
mod input_descriptor_tests;

#[cfg(test)]
mod presentation_tests;

#[cfg(test)]
mod enum_labels_tests;
