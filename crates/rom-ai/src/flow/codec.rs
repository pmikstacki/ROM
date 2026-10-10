//! Private bounded canonical storage and action codecs; errors never include input.
use crate::{AiError, AiResult};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub(crate) const MAX_RECORD_BYTES: usize = 256 * 1024;
pub(crate) trait Validated {
    fn validate(&self) -> AiResult<()>;
}
pub(crate) fn encode<T: Serialize + Validated>(value: &T) -> AiResult<String> {
    value.validate()?;
    let encoded = serde_json::to_string(value)?;
    if encoded.len() > MAX_RECORD_BYTES {
        return Err(AiError::BudgetExhausted);
    }
    Ok(encoded)
}
pub(crate) fn decode<T: DeserializeOwned + Validated>(encoded: &str) -> AiResult<T> {
    if encoded.len() > MAX_RECORD_BYTES {
        return Err(AiError::InvalidRequest);
    }
    let value: T = serde_json::from_str(encoded)?;
    value.validate()?;
    Ok(value)
}
pub(crate) fn rom_error(error: AiError) -> rom::Error {
    match error {
        AiError::Denied => rom::Error::Denied,
        AiError::Conflict => rom::Error::Conflict,
        _ => rom::Error::invalid("ai", "contract"),
    }
}
pub(crate) fn input_decode<T: DeserializeOwned + Validated>(value: Value) -> rom::Result<T> {
    let encoded = serde_json::to_string(&value).map_err(|_| rom::Error::invalid("ai", "input"))?;
    decode(&encoded).map_err(rom_error)
}
macro_rules! input {
    ($type:ty) => {
        impl rom::Input for $type {
            fn encode(&self) -> serde_json::Value {
                serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
            }
            fn decode(value: serde_json::Value) -> rom::Result<Self> {
                $crate::flow::codec::input_decode(value)
            }
        }
    };
}
pub(crate) use input;
