//! config-rs is a parser here; ROM owns identity, ownership and publication.
use config::{File, FileFormat, Source, ValueKind};
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use std::collections::{BTreeMap, BTreeSet};

/// Explicit format; there is no discovery, environment scan or optional file fallback.
#[derive(Clone, Copy, Debug)]
pub enum Format {
    Json,
    Toml,
}
/// Safe error classes. Raw loader errors and supplied values are never retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    TooLarge,
    InvalidOrigin,
    Parse,
    Ambiguous,
    NonFinite,
    InvalidValue,
}
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "configuration rejected: {self:?}")
    }
}
impl std::error::Error for ConfigError {}

/// Parsed fields, not an accepted Resource or a committed configuration.
pub struct ParsedDocument {
    values: rom::Value,
    origins: BTreeMap<String, String>,
}
impl ParsedDocument {
    /// Values must still pass the registered Resource's normal codec and policies.
    pub fn values(&self) -> &rom::Value {
        &self.values
    }
    /// Safe host labels per top-level field; no filesystem path or value is included.
    pub fn origins(&self) -> &BTreeMap<String, String> {
        &self.origins
    }
}
impl std::fmt::Debug for ParsedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedDocument")
            .field("field_count", &self.origins.len())
            .finish_non_exhaustive()
    }
}

/// Parse at most 64 KiB and 128 top-level fields. Identity and grants are supplied
/// separately by the host; this function does not register definitions or merge layers.
pub fn parse(
    format: Format,
    document: &str,
    safe_origin: &str,
) -> Result<ParsedDocument, ConfigError> {
    if document.len() > 65_536 {
        return Err(ConfigError::TooLarge);
    }
    if safe_origin.is_empty()
        || safe_origin.len() > 64
        || !safe_origin
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(ConfigError::InvalidOrigin);
    }
    if matches!(format, Format::Json) {
        let mut deserializer = serde_json::Deserializer::from_str(document);
        NoDuplicates::deserialize(&mut deserializer).map_err(|e| {
            if e.classify() == serde_json::error::Category::Data {
                ConfigError::Ambiguous
            } else {
                ConfigError::Parse
            }
        })?;
        deserializer.end().map_err(|_| ConfigError::Parse)?;
        if !document.trim_start().starts_with('{') {
            return Err(ConfigError::InvalidValue);
        }
    }
    let format = match format {
        Format::Json => FileFormat::Json,
        Format::Toml => FileFormat::Toml,
    };
    // Collect directly: Config::builder would interpret literal dotted keys as paths.
    let collected = File::from_str(document, format)
        .collect()
        .map_err(|_| ConfigError::Parse)?;
    if collected.len() > 128 {
        return Err(ConfigError::TooLarge);
    }
    let mut values = rom::Map::new();
    let mut origins = BTreeMap::new();
    for (field, value) in collected {
        // String-backed File sources have no trusted path. The host supplies this
        // safe provenance label before values lose their config::Value wrapper.
        origins.insert(field.clone(), safe_origin.to_owned());
        values.insert(field, to_json(value.kind)?);
    }
    Ok(ParsedDocument {
        values: rom::Value::Object(values),
        origins,
    })
}
fn to_json(kind: ValueKind) -> Result<rom::Value, ConfigError> {
    Ok(match kind {
        ValueKind::Nil => rom::Value::Null,
        ValueKind::Boolean(v) => rom::json!(v),
        ValueKind::I64(v) => rom::json!(v),
        ValueKind::U64(v) => rom::json!(v),
        ValueKind::I128(v) => rom::json!(i64::try_from(v).map_err(|_| ConfigError::InvalidValue)?),
        ValueKind::U128(v) => rom::json!(u64::try_from(v).map_err(|_| ConfigError::InvalidValue)?),
        ValueKind::Float(v) => {
            rom::Value::Number(serde_json::Number::from_f64(v).ok_or(ConfigError::NonFinite)?)
        }
        ValueKind::String(v) => rom::Value::String(v),
        ValueKind::Array(v) => rom::Value::Array(
            v.into_iter()
                .map(|v| to_json(v.kind))
                .collect::<Result<_, _>>()?,
        ),
        ValueKind::Table(v) => rom::Value::Object(
            v.into_iter()
                .map(|(k, v)| Ok((k, to_json(v.kind)?)))
                .collect::<Result<_, ConfigError>>()?,
        ),
    })
}

// JSON objects otherwise silently keep only the last duplicate key, hiding an
// invalid contribution before Resource validation. Inspect syntax recursively.
struct NoDuplicates;
impl<'de> Deserialize<'de> for NoDuplicates {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Check;
        impl<'de> Visitor<'de> for Check {
            type Value = NoDuplicates;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unambiguous JSON")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut seen = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key) {
                        return Err(de::Error::custom("duplicate field"));
                    }
                    map.next_value::<NoDuplicates>()?;
                }
                Ok(NoDuplicates)
            }
            fn visit_seq<S: SeqAccess<'de>>(
                self,
                mut sequence: S,
            ) -> Result<Self::Value, S::Error> {
                while sequence.next_element::<NoDuplicates>()?.is_some() {}
                Ok(NoDuplicates)
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(NoDuplicates)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(NoDuplicates)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(NoDuplicates)
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(NoDuplicates)
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(NoDuplicates)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicates)
            }
        }
        d.deserialize_any(Check)
    }
}
