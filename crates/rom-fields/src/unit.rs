use crate::{Decimal, scalar::invalid};
use rom::{CodecIdentity, Field, Result, Shape, Value, json};
/// Exact magnitude and explicit opaque unit token. No conversion or unit inference occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitValue {
    value: Decimal,
    unit: String,
}
impl UnitValue {
    /// Unit tokens contain 1..64 ASCII letters, digits, `_`, `-`, `/`, `.`, `%`, or `^`.
    pub fn new(value: Decimal, unit: impl Into<String>) -> Result<Self> {
        let unit = unit.into();
        if unit.is_empty()
            || unit.len() > 64
            || !unit
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-/.%^".contains(&b))
        {
            return Err(invalid("rom.unit-value"));
        }
        Ok(Self { value, unit })
    }
    pub fn value(&self) -> &Decimal {
        &self.value
    }
    pub fn unit(&self) -> &str {
        &self.unit
    }
}
impl Field for UnitValue {
    fn shape() -> Shape {
        Shape::Map(Box::new(Shape::String))
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "rom.unit-value".into(),
            version: 1,
        })
    }
    fn encode(&self) -> Value {
        json!({"value": self.value.as_str(), "unit": self.unit})
    }
    fn decode(value: Value) -> Result<Self> {
        let Value::Object(mut values) = value else {
            return Err(invalid("rom.unit-value"));
        };
        if values.len() != 2 {
            return Err(invalid("rom.unit-value"));
        }
        let value = Decimal::decode(
            values
                .remove("value")
                .ok_or_else(|| invalid("rom.unit-value"))?,
        )?;
        let Value::String(unit) = values
            .remove("unit")
            .ok_or_else(|| invalid("rom.unit-value"))?
        else {
            return Err(invalid("rom.unit-value"));
        };
        Self::new(value, unit)
    }
}
