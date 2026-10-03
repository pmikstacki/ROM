//! Identifiers and bounded labels from trusted host inputs, never credential bytes.
use rom::{Error, Result};

pub(super) fn bounded(value: &str, limit: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > limit
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        Err(Error::Denied)
    } else {
        Ok(())
    }
}
