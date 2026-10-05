//! Enum labels are advisory. Stored values remain the declared canonical members.
use rom::{Field, Result, Shape, Value};
use std::collections::BTreeMap;

#[derive(Clone)]
pub enum Category {
    Inspection,
    Maintenance,
    Reporting,
}
impl Field for Category {
    fn shape() -> Shape {
        Shape::Enum(vec![
            "safety_check".into(),
            "preventive_maintenance".into(),
            "reporting".into(),
        ])
    }
    fn enum_labels() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("safety_check".into(), "Safety inspection".into()),
            (
                "preventive_maintenance".into(),
                "Preventive maintenance".into(),
            ),
            ("reporting".into(), "Reporting".into()),
        ])
    }
    fn encode(&self) -> Value {
        match self {
            Self::Inspection => "safety_check",
            Self::Maintenance => "preventive_maintenance",
            Self::Reporting => "reporting",
        }
        .into()
    }
    fn decode(value: Value) -> Result<Self> {
        match value.as_str() {
            Some("safety_check") => Ok(Self::Inspection),
            Some("preventive_maintenance") => Ok(Self::Maintenance),
            Some("reporting") => Ok(Self::Reporting),
            _ => Err(rom::Error::invalid(
                "category",
                "declared category required",
            )),
        }
    }
}
