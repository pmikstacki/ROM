//! Authoring metadata for codecs and action inputs, separate from persisted layouts.
use crate::{Error, Result, Shape, validate_shape};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A trusted custom codec's presentation identity, scoped to its Resource binding.
/// This identifies a codec, not a global Field registry or executable plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodecIdentity {
    pub name: String,
    pub version: u32,
}
impl CodecIdentity {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.name.is_empty() || self.name.len() > 256 || self.version == 0 {
            return Err(Error::invalid("input", "codec identity"));
        }
        Ok(())
    }
}

/// Built-in wrappers from an outer field to its custom codec leaf.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecWrapper {
    Optional,
    Nullable,
    List,
    Map,
}

/// One Resource-owned custom codec binding. This is not persisted catalog identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldCodec {
    pub name: String,
    pub codec: CodecIdentity,
    pub codec_wrappers: Vec<CodecWrapper>,
}

/// A named input member described by the same typed codec as its wire value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFieldDescriptor {
    pub name: String,
    pub shape: Shape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec: Option<CodecIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codec_wrappers: Vec<CodecWrapper>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub enum_labels: BTreeMap<String, String>,
}

/// An action input's supported wire representation. `None` describes an opaque input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum InputDescriptor {
    Unit,
    Scalar {
        shape: Shape,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        codec: Option<CodecIdentity>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        codec_wrappers: Vec<CodecWrapper>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        enum_labels: BTreeMap<String, String>,
    },
    Object(Vec<InputFieldDescriptor>),
}
impl InputDescriptor {
    pub(crate) fn validate(&self, kinds: Option<&BTreeSet<String>>) -> Result<()> {
        match self {
            Self::Unit => Ok(()),
            Self::Scalar {
                shape,
                codec,
                codec_wrappers,
                enum_labels,
            } => {
                // Scalar action input has a value; absence only describes object members.
                if matches!(shape, Shape::Optional(_)) {
                    return Err(Error::invalid("input", "scalar presence"));
                }
                validate_shape(shape, 0, kinds)?;
                super::enum_labels::validate_labels(shape, enum_labels)?;
                validate_wrappers(shape, codec.as_ref(), codec_wrappers)?;
                if let Some(codec) = codec {
                    codec.validate()?;
                }
                Ok(())
            }
            Self::Object(fields) => {
                let mut names = BTreeSet::new();
                for field in fields {
                    if field.name.is_empty() || !names.insert(&field.name) {
                        return Err(Error::invalid("input", "duplicate or empty field"));
                    }
                    validate_shape(&field.shape, 0, kinds)?;
                    super::enum_labels::validate_labels(&field.shape, &field.enum_labels)?;
                    validate_wrappers(&field.shape, field.codec.as_ref(), &field.codec_wrappers)?;
                    if let Some(codec) = &field.codec {
                        codec.validate()?;
                    }
                }
                Ok(())
            }
        }
    }
    pub(crate) fn visible(&self, kinds: &BTreeSet<&str>) -> bool {
        match self {
            Self::Unit => true,
            Self::Scalar { shape, .. } => visible_shape(shape, kinds),
            Self::Object(fields) => fields
                .iter()
                .all(|field| visible_shape(&field.shape, kinds)),
        }
    }
}
pub(crate) fn visible_shape(shape: &Shape, kinds: &BTreeSet<&str>) -> bool {
    match shape {
        Shape::Reference { kind } => kinds.contains(kind.as_str()),
        Shape::Nullable(inner)
        | Shape::Optional(inner)
        | Shape::List(inner)
        | Shape::Map(inner) => visible_shape(inner, kinds),
        _ => true,
    }
}

pub(crate) fn field_bindings(
    descriptor: &crate::Descriptor,
    bindings: Vec<FieldCodec>,
) -> Result<std::collections::BTreeMap<String, FieldCodec>> {
    let mut result = std::collections::BTreeMap::new();
    for binding in bindings {
        binding.codec.validate()?;
        let field = descriptor
            .fields
            .iter()
            .find(|field| field.name == binding.name)
            .ok_or_else(|| Error::invalid(&descriptor.kind, "codec binding"))?;
        validate_wrappers(&field.shape, Some(&binding.codec), &binding.codec_wrappers)?;
        if result.insert(binding.name.clone(), binding).is_some() {
            return Err(Error::invalid(&descriptor.kind, "codec binding"));
        }
    }
    Ok(result)
}

fn validate_wrappers(
    shape: &Shape,
    codec: Option<&CodecIdentity>,
    wrappers: &[CodecWrapper],
) -> Result<()> {
    if wrappers.len() > 16 || codec.is_none() && !wrappers.is_empty() {
        return Err(Error::invalid("input", "codec wrapper path"));
    }
    let mut shape = shape;
    for wrapper in wrappers {
        shape = match (wrapper, shape) {
            (CodecWrapper::Optional, Shape::Optional(inner))
            | (CodecWrapper::Nullable, Shape::Nullable(inner))
            | (CodecWrapper::List, Shape::List(inner))
            | (CodecWrapper::Map, Shape::Map(inner)) => inner,
            _ => return Err(Error::invalid("input", "codec wrapper path")),
        };
    }
    Ok(())
}
