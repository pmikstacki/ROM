//! Validated semantic fields with immutable version-one wire contracts.
//! Constructors and `Field::decode` use the same validation. Compose these fields
//! with ROM's existing nullable, optional, list, map, and typed reference wrappers.
mod decimal;
mod scalar;
mod temporal;
mod text;
mod unit;

pub use decimal::Decimal;
pub use rom::ResourceRef;
pub use temporal::{Date, DateTime, Time};
pub use text::{Color, Email, JsonDocument, Multiline, Url};
pub use unit::UnitValue;
