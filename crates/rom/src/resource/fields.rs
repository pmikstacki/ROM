//! Scalar and container codecs for Resource fields and action inputs.
use crate::{Error, Resource, Result, Shape, Value, json, validate_shape};
use std::{collections::BTreeMap, marker::PhantomData};

pub trait Field: Clone + Send + Sync + 'static {
    fn shape() -> Shape;
    /// Optional codec identity for descriptor-driven presentation.
    fn codec_identity() -> Option<crate::CodecIdentity> {
        None
    }
    /// Built-in wrapper path to the leaf presentation codec. Custom codecs own their shape by default.
    fn codec_wrappers() -> Vec<crate::CodecWrapper> {
        Vec::new()
    }
    /// Describe standalone input encoding; override when it differs from field encoding.
    fn input_descriptor() -> Option<crate::InputDescriptor> {
        Some(crate::InputDescriptor::Scalar {
            shape: Self::shape(),
            codec: Self::codec_identity(),
            codec_wrappers: Self::codec_wrappers(),
        })
    }
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
    fn is_present(&self) -> bool {
        true
    }
    fn decode_missing() -> Result<Self> {
        Err(Error::invalid("input", "missing field"))
    }
    fn encode_input(&self) -> Value {
        self.encode()
    }
    fn decode_input(value: Value) -> Result<Self> {
        Self::decode(value)
    }
}
macro_rules! scalar {
    ($ty:ty,$shape:ident,$get:ident,$map:expr) => {
        impl Field for $ty {
            fn shape() -> Shape {
                Shape::$shape
            }
            fn encode(&self) -> Value {
                json!(self)
            }
            fn decode(value: Value) -> Result<Self> {
                value
                    .$get()
                    .map($map)
                    .ok_or_else(|| Error::invalid("input", "$"))
            }
        }
    };
}
scalar!(String, String, as_str, |s: &str| s.to_owned());
scalar!(bool, Bool, as_bool, |v| v);
scalar!(u64, U64, as_u64, |v| v);
scalar!(i64, I64, as_i64, |v| v);
/// A finite JSON floating-point value. Invalid floats cannot become a nullable null.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiniteF64(f64);
impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(Error::invalid("input", "finite number"))
        }
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
impl TryFrom<f64> for FiniteF64 {
    type Error = Error;
    fn try_from(value: f64) -> Result<Self> {
        Self::new(value)
    }
}
impl Field for FiniteF64 {
    fn shape() -> Shape {
        Shape::F64
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_f64()
            .ok_or_else(|| Error::invalid("input", "finite number"))
            .and_then(Self::new)
    }
}
impl<T: Field> Field for Vec<T> {
    fn codec_identity() -> Option<crate::CodecIdentity> {
        T::codec_identity()
    }
    fn codec_wrappers() -> Vec<crate::CodecWrapper> {
        wrapper_path::<T>(crate::CodecWrapper::List)
    }
    fn shape() -> Shape {
        Shape::List(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        Value::Array(self.iter().map(Field::encode).collect())
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::Array(values) => values.into_iter().map(T::decode).collect(),
            _ => Err(Error::invalid("input", "$")),
        }
    }
}
impl<T: Field> Field for BTreeMap<String, T> {
    fn codec_identity() -> Option<crate::CodecIdentity> {
        T::codec_identity()
    }
    fn codec_wrappers() -> Vec<crate::CodecWrapper> {
        wrapper_path::<T>(crate::CodecWrapper::Map)
    }
    fn shape() -> Shape {
        Shape::Map(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        Value::Object(self.iter().map(|(k, v)| (k.clone(), v.encode())).collect())
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::Object(values) => values
                .into_iter()
                .map(|(k, v)| Ok((k, T::decode(v)?)))
                .collect(),
            _ => Err(Error::invalid("input", "$")),
        }
    }
}
/// A typed reference. Persistence enforces target existence and restrict deletion.
#[derive(Clone)]
pub struct ResourceRef<R: Resource> {
    id: String,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> ResourceRef<R> {
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.is_empty() {
            return Err(Error::invalid(R::KIND, "reference identity"));
        }
        Ok(Self {
            id,
            marker: PhantomData,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}
impl<R: Resource> std::fmt::Debug for ResourceRef<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceRef")
            .field("kind", &R::KIND)
            .field("id", &self.id)
            .finish()
    }
}
impl<R: Resource> PartialEq for ResourceRef<R> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<R: Resource> Eq for ResourceRef<R> {}
impl<R: Resource> Field for ResourceRef<R> {
    fn shape() -> Shape {
        Shape::Reference {
            kind: R::KIND.into(),
        }
    }
    fn encode(&self) -> Value {
        Value::String(self.id.clone())
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::String(id) => Self::new(id),
            _ => Err(Error::invalid(R::KIND, "reference identity")),
        }
    }
}
impl<T: Field> Field for Option<T> {
    fn codec_identity() -> Option<crate::CodecIdentity> {
        T::codec_identity()
    }
    fn codec_wrappers() -> Vec<crate::CodecWrapper> {
        wrapper_path::<T>(crate::CodecWrapper::Nullable)
    }
    fn shape() -> Shape {
        Shape::Nullable(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        self.as_ref().map_or(Value::Null, Field::encode)
    }
    fn decode(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(None)
        } else {
            T::decode(v).map(Some)
        }
    }
}
pub trait Input: Clone + Send + Sync + 'static {
    /// Describe this input from its actual codec. Manual inputs are opaque by default.
    fn descriptor() -> Option<crate::InputDescriptor> {
        None
    }
    /// Declared object member names allowed in safe action validation diagnostics.
    /// Scalar/manual inputs default to action-level errors. This does not register a Resource.
    fn field_names() -> &'static [&'static str] {
        &[]
    }
    fn encode(&self) -> Value;
    fn decode(v: Value) -> Result<Self>;
}
impl<T: Field> Input for T {
    fn descriptor() -> Option<crate::InputDescriptor> {
        T::input_descriptor()
    }
    fn encode(&self) -> Value {
        Field::encode_input(self)
    }
    fn decode(v: Value) -> Result<Self> {
        validate_shape(&T::shape(), 0, None)?;
        T::decode_input(v)
    }
}
impl Input for () {
    fn descriptor() -> Option<crate::InputDescriptor> {
        Some(crate::InputDescriptor::Unit)
    }
    fn encode(&self) -> Value {
        Value::Null
    }
    fn decode(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(())
        } else {
            Err(Error::invalid("input", "$"))
        }
    }
}

pub(crate) fn wrapper_path<T: Field>(outer: crate::CodecWrapper) -> Vec<crate::CodecWrapper> {
    if T::codec_identity().is_none() {
        return Vec::new();
    }
    let mut path = vec![outer];
    path.extend(T::codec_wrappers());
    path
}
