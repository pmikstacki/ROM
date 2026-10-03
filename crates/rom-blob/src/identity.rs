//! Content identity and opaque provider keys.
use crate::{Error, Result};
use rom::{Field, Value};
use sha2::{Digest as _, Sha256};

/// SHA-256 content identity, independent of provider ETags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digest(String);
impl Digest {
    pub fn of(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() != 64
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Invalid);
        }
        Ok(Self(text.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Field for Digest {
    fn shape() -> rom::Shape {
        rom::Shape::String
    }
    fn encode(&self) -> Value {
        rom::json!(self.0)
    }
    fn decode(value: Value) -> rom::Result<Self> {
        value
            .as_str()
            .and_then(|s| Self::parse(s).ok())
            .ok_or_else(|| rom::Error::invalid("blobs", "digest"))
    }
}
/// Opaque physical key. It conveys no authority; only trusted adapters consume it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectKey(pub(crate) Digest);
impl ObjectKey {
    pub fn parse(text: &str) -> Result<Self> {
        Ok(Self(Digest::parse(text)?))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
