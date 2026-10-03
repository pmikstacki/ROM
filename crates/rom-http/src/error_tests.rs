use crate::error::{Failure, category};
use axum::{http::StatusCode, response::IntoResponse};
use rom::Error;

#[tokio::test]
async fn expired_identity_has_a_distinct_gone_category() {
    assert_eq!(
        category(&Error::IdentityExpired),
        ("identity_expired", StatusCode::GONE)
    );
    let response = Failure(Error::IdentityExpired).into_response();
    assert_eq!(response.status(), StatusCode::GONE);
    let bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        serde_json::json!({"error":"identity_expired"})
    );
}
