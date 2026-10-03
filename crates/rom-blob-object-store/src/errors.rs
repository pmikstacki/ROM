//! Preserve the distinction between read failures and uncertain writes.
use rom_blob::Error;

pub(crate) fn read_error(error: object_store::Error) -> Error {
    match error {
        object_store::Error::NotFound { .. } => Error::Missing,
        object_store::Error::PermissionDenied { .. } => Error::Denied,
        object_store::Error::NotImplemented { .. } | object_store::Error::NotSupported { .. } => {
            Error::Unsupported
        }
        _ => Error::Backend,
    }
}
pub(crate) fn write_error(error: object_store::Error) -> Error {
    match error {
        object_store::Error::AlreadyExists { .. } | object_store::Error::Precondition { .. } => {
            Error::Conflict
        }
        object_store::Error::PermissionDenied { .. } => Error::Denied,
        object_store::Error::NotImplemented { .. } | object_store::Error::NotSupported { .. } => {
            Error::Unsupported
        }
        _ => Error::Unknown,
    }
}
