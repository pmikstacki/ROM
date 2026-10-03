//! Resource descriptors and canonical shape validation.
use crate::{Definition, Error, Map, Result, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Shape {
    String,
    Bool,
    U64,
    I64,
    F64,
    Nullable(Box<Shape>),
    Optional(Box<Shape>),
    List(Box<Shape>),
    Map(Box<Shape>),
    Enum(Vec<String>),
    Reference { kind: String },
}

impl Shape {
    /// Whether values use scalar query semantics, including missing and null wrappers.
    /// Lists and maps retain structural equality and are not scalar index keys.
    pub fn is_scalar(&self) -> bool {
        match self {
            Self::Optional(inner) | Self::Nullable(inner) => inner.is_scalar(),
            Self::List(_) | Self::Map(_) => false,
            _ => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDescriptor {
    pub name: String,
    pub shape: Shape,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub kind: String,
    pub version: u32,
    pub fields: Vec<FieldDescriptor>,
}
pub trait Resource: Clone + Send + Sync + 'static {
    const KIND: &'static str;
    fn descriptor() -> Descriptor;
    fn normalize_field(name: &str, value: Value) -> Result<Value>;
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
    fn definition() -> Definition<Self> {
        Definition::new()
    }
}

pub(crate) fn canonical_fields<'a>(
    descriptor: &Descriptor,
    value: &'a Value,
) -> Result<&'a Map<String, Value>> {
    let map = value
        .as_object()
        .ok_or_else(|| Error::invalid(&descriptor.kind, "codec object"))?;
    if map
        .keys()
        .any(|key| !descriptor.fields.iter().any(|f| &f.name == key))
    {
        return Err(Error::invalid(&descriptor.kind, "codec fields"));
    }
    for field in &descriptor.fields {
        if !map
            .get(&field.name)
            .map_or(matches!(field.shape, Shape::Optional(_)), |v| {
                matches_shape(v, &field.shape)
            })
        {
            return Err(Error::invalid(&descriptor.kind, &field.name));
        }
    }
    Ok(map)
}

pub(crate) fn matches_shape(value: &Value, shape: &Shape) -> bool {
    match shape {
        Shape::String => value.is_string(),
        Shape::Bool => value.is_boolean(),
        Shape::U64 => value.as_u64().is_some(),
        Shape::I64 => value.as_i64().is_some(),
        Shape::F64 => value.as_f64().is_some_and(f64::is_finite),
        Shape::Nullable(inner) => value.is_null() || matches_shape(value, inner),
        Shape::Optional(inner) => matches_shape(value, inner),
        Shape::List(inner) => value
            .as_array()
            .is_some_and(|vs| vs.iter().all(|v| matches_shape(v, inner))),
        Shape::Map(inner) => value
            .as_object()
            .is_some_and(|vs| vs.values().all(|v| matches_shape(v, inner))),
        Shape::Enum(variants) => value
            .as_str()
            .is_some_and(|v| variants.iter().any(|allowed| v == allowed)),
        Shape::Reference { .. } => value.as_str().is_some_and(|v| !v.is_empty()),
    }
}

pub(crate) fn validate_shape(
    shape: &Shape,
    depth: usize,
    kinds: Option<&BTreeSet<String>>,
) -> Result<()> {
    if depth > 16 {
        return Err(Error::Unsupported("field shape nesting exceeds 16".into()));
    }
    match shape {
        Shape::Optional(inner) => {
            if depth != 0 {
                return Err(Error::Unsupported(
                    "presence is only valid on a top-level Resource field".into(),
                ));
            }
            validate_shape(inner, depth + 1, kinds)
        }
        Shape::Nullable(inner) => {
            if matches!(inner.as_ref(), Shape::Nullable(_)) {
                return Err(Error::Unsupported(
                    "nested nullable has no unambiguous codec".into(),
                ));
            }
            validate_shape(inner, depth + 1, kinds)
        }
        Shape::List(inner) | Shape::Map(inner) => validate_shape(inner, depth + 1, kinds),
        Shape::Enum(variants) => {
            let unique: BTreeSet<_> = variants.iter().collect();
            if variants.is_empty()
                || variants.len() > 256
                || unique.len() != variants.len()
                || variants.iter().any(String::is_empty)
            {
                Err(Error::Unsupported(
                    "enum requires 1..=256 distinct nonempty values".into(),
                ))
            } else {
                Ok(())
            }
        }
        Shape::Reference { kind } => {
            if kind.is_empty() || kinds.is_some_and(|ks| !ks.contains(kind)) {
                Err(Error::Unsupported(format!(
                    "unregistered reference target: {kind}"
                )))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}
