//! Exact scalar keys for the version-1 native candidate profile.
use rom::{Error, Result, Shape, Value};

/// No codec callback runs here. Values have the registered canonical field shape.
/// Prefixes order missing before null before present values; all keys bind as BLOBs.
pub(crate) fn encode(shape: &Shape, value: Option<&Value>) -> Result<Vec<u8>> {
    if !shape.is_scalar() {
        return Err(Error::Unsupported("non-scalar query key".into()));
    }
    match value {
        None if matches!(shape, Shape::Optional(_)) => Ok(vec![0]),
        None => Err(Error::Storage),
        Some(value) => present(shape, value),
    }
}
fn present(shape: &Shape, value: &Value) -> Result<Vec<u8>> {
    match shape {
        Shape::Optional(inner) => return present(inner, value),
        Shape::Nullable(_) if value.is_null() => return Ok(vec![1]),
        Shape::Nullable(inner) => return present(inner, value),
        _ => {}
    }
    let mut key = vec![2];
    match shape {
        Shape::U64 => key.extend(value.as_u64().ok_or(Error::Storage)?.to_be_bytes()),
        Shape::I64 => {
            key.extend(((value.as_i64().ok_or(Error::Storage)? as u64) ^ (1 << 63)).to_be_bytes())
        }
        Shape::F64 => {
            let number = value
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or(Error::Storage)?;
            // Core's partial comparison equates both zeros and integer/float representations.
            let bits = if number == 0.0 { 0 } else { number.to_bits() };
            let ordered = if bits & (1 << 63) != 0 {
                !bits
            } else {
                bits ^ (1 << 63)
            };
            key.extend(ordered.to_be_bytes());
        }
        Shape::Bool => key.push(u8::from(value.as_bool().ok_or(Error::Storage)?)),
        Shape::String => key.extend(value.as_str().ok_or(Error::Storage)?.as_bytes()),
        Shape::Enum(variants) => {
            let text = value.as_str().ok_or(Error::Storage)?;
            if !variants.iter().any(|v| v == text) {
                return Err(Error::Storage);
            }
            key.extend(text.as_bytes());
        }
        Shape::Reference { .. } => {
            let text = value
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or(Error::Storage)?;
            key.extend(text.as_bytes());
        }
        _ => return Err(Error::Storage),
    }
    Ok(key)
}
