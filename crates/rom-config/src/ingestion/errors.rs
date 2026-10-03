//! Safe public reload failures, with dynamic target validation details redacted.
use crate::ConfigError;
use rom::Error;

pub(super) fn safe_runtime_error(error: Error) -> Error {
    match error {
        Error::Invalid { .. } => Error::invalid("configuration-target", "candidate"),
        Error::Unsupported(_) => Error::Unsupported("configuration target capability".into()),
        Error::Duplicate(_) => Error::Duplicate("configuration target".into()),
        other => other,
    }
}
/// Structured safe failure; dynamic target validation details are redacted.
#[derive(Debug)]
pub enum ReloadError {
    Parse(ConfigError),
    Runtime(Error),
}
impl std::fmt::Display for ReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => e.fmt(f),
            Self::Runtime(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ReloadError {}
