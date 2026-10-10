//! Codec-derived JSON schema; typed decode remains authoritative.
use crate::{AiError, AiResult};
use rom::{InputDescriptor, Shape};
use serde_json::{Value, json};

pub(crate) fn input<I: rom::Input>() -> AiResult<Value> {
    let value = match I::descriptor().ok_or(AiError::UnsupportedCapability)? {
        InputDescriptor::Unit => json!({"type":"null"}),
        InputDescriptor::Scalar { shape: value, .. } => shape(&value, 0)?,
        InputDescriptor::Object(fields) => {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();
            if fields.len() > 256 {
                return Err(AiError::InvalidRequest);
            }
            for field in fields {
                if field.name.is_empty()
                    || field.name.len() > 256
                    || properties.contains_key(&field.name)
                {
                    return Err(AiError::InvalidRequest);
                }
                if !matches!(field.shape, Shape::Optional(_)) {
                    required.push(field.name.clone());
                }
                properties.insert(field.name, shape(&field.shape, 0)?);
            }
            json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
        }
    };
    crate::request::bounded_value(&value, crate::request::MAX_TOOL_BYTES)?;
    Ok(value)
}
fn shape(value: &Shape, depth: u32) -> AiResult<Value> {
    if depth > 32 {
        return Err(AiError::InvalidRequest);
    }
    Ok(match value {
        Shape::String => json!({"type":"string"}),
        Shape::Bool => json!({"type":"boolean"}),
        Shape::U64 => json!({"type":"integer","minimum":0,"maximum":u64::MAX}),
        Shape::I64 => json!({"type":"integer","minimum":i64::MIN,"maximum":i64::MAX}),
        Shape::F64 => json!({"type":"number"}),
        Shape::Nullable(inner) => json!({"anyOf":[shape(inner, depth + 1)?,{"type":"null"}]}),
        Shape::Optional(inner) => shape(inner, depth + 1)?,
        Shape::List(inner) => json!({"type":"array","items":shape(inner, depth + 1)?}),
        Shape::Map(inner) => {
            json!({"type":"object","additionalProperties":shape(inner, depth + 1)?})
        }
        Shape::Enum(values) => {
            if values.is_empty()
                || values.len() > 256
                || values.iter().any(|value| value.len() > 256)
            {
                return Err(AiError::InvalidRequest);
            }
            json!({"type":"string","enum":values})
        }
        Shape::Reference { .. } => json!({"type":"string"}),
    })
}
