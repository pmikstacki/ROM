use rom::{CodecIdentity, Error, Field, Result, Shape, Value};
use rom_fields::JsonDocument;
use std::collections::BTreeSet;

const INITIAL: &str = r#"[{"id":"history","x":0,"y":0,"width":2,"height":1,"visible":true}]"#;

/// Application-owned, bounded presentation stored through the ordinary Settings Resource.
#[derive(Clone)]
pub struct WorkspaceLayout(JsonDocument);

impl WorkspaceLayout {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.len() > 8192 {
            return Err(invalid());
        }
        let layout = Self(JsonDocument::new(value)?);
        layout.validate_for(4)?;
        Ok(layout)
    }

    pub fn initial() -> Result<Self> {
        Self::new(INITIAL)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn validate_for(&self, columns: u64) -> Result<()> {
        if !(1..=4).contains(&columns) {
            return Err(invalid());
        }
        let decoded = rom::parse_json(self.as_str().as_bytes())?;
        let items = decoded.as_array().ok_or_else(invalid)?;
        if items.len() > 3 {
            return Err(invalid());
        }
        let mut ids = BTreeSet::new();
        for item in items {
            let values = item.as_object().ok_or_else(invalid)?;
            if values.len() != 6 {
                return Err(invalid());
            }
            let id = values
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            if !matches!(id, "history" | "equipment" | "work") || !ids.insert(id) {
                return Err(invalid());
            }
            let number = |name: &str| values.get(name).and_then(Value::as_u64).ok_or_else(invalid);
            let (x, y, width, height) = (
                number("x")?,
                number("y")?,
                number("width")?,
                number("height")?,
            );
            if !(1..=4).contains(&width)
                || !(1..=4).contains(&height)
                || x.checked_add(width).is_none_or(|end| end > columns)
                || y.checked_add(height).is_none_or(|end| end > 8)
                || values.get("visible").and_then(Value::as_bool).is_none()
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

impl Field for WorkspaceLayout {
    fn shape() -> Shape {
        JsonDocument::shape()
    }
    fn codec_identity() -> Option<CodecIdentity> {
        JsonDocument::codec_identity()
    }
    fn encode(&self) -> Value {
        self.0.encode()
    }
    fn decode(value: Value) -> Result<Self> {
        match value {
            Value::String(value) => Self::new(value),
            _ => Err(invalid()),
        }
    }
}

fn invalid() -> Error {
    Error::invalid("portal-settings", "layout")
}
