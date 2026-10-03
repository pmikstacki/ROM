use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rom::Error;

pub(super) struct Failure(pub(super) Error);
impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
pub(super) fn category(e: &Error) -> (&'static str, StatusCode) {
    match e {
        Error::Denied => ("denied", StatusCode::FORBIDDEN),
        Error::Missing | Error::Unregistered => ("missing", StatusCode::NOT_FOUND),
        Error::Conflict => ("conflict", StatusCode::CONFLICT),
        Error::IdentityMismatch => ("identity_mismatch", StatusCode::CONFLICT),
        Error::IdentityExpired => ("identity_expired", StatusCode::GONE),
        Error::HistoryGap => ("history_gap", StatusCode::GONE),
        Error::TooLarge => ("too_large", StatusCode::PAYLOAD_TOO_LARGE),
        Error::Overloaded => ("overloaded", StatusCode::TOO_MANY_REQUESTS),
        Error::Closed => ("closed", StatusCode::SERVICE_UNAVAILABLE),
        Error::Unknown => ("outcome_unknown", StatusCode::SERVICE_UNAVAILABLE),
        Error::NotCommitted => ("not_committed", StatusCode::SERVICE_UNAVAILABLE),
        Error::Invalid { .. } | Error::Unsupported(_) | Error::Duplicate(_) => {
            ("invalid", StatusCode::BAD_REQUEST)
        }
        _ => ("internal", StatusCode::INTERNAL_SERVER_ERROR),
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (code, status) = category(&self.0);
        (status, Json(serde_json::json!({"error":code}))).into_response()
    }
}
