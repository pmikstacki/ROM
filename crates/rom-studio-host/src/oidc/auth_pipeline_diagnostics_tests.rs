//! Actual protected HTTP requests expose both trusted resolver lanes and pre-resolve denial.
use super::credentials_tests::Fixture;
use crate::{AuthOperation, AuthStage};

async fn request(fixture: &Fixture, csrf: &str) -> reqwest::StatusCode {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = fixture.host.router();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    let response = client
        .post(format!("http://{address}/rom-studio/api/discover"))
        .header("origin", "http://127.0.0.1:43210")
        .header("cookie", format!("rom_session={}", fixture.cookie))
        .header("x-rom-csrf", csrf)
        .body("{}")
        .send()
        .await
        .unwrap();
    let status = response.status();
    let _ = response.bytes().await.unwrap();
    drop(client);
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    status
}

#[tokio::test]
async fn ordinary_http_records_two_separate_accepted_authentication_lanes() {
    let fixture = Fixture::observed(200, false).await;
    let csrf = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 100)
        .unwrap()
        .csrf()
        .to_owned();
    assert_eq!(request(&fixture, &csrf).await, reqwest::StatusCode::OK);
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(
        snapshot
            .operation(AuthOperation::ProtectedMiddleware)
            .admitted,
        1
    );
    assert_eq!(
        snapshot
            .operation(AuthOperation::GenericHttpResolver)
            .admitted,
        1
    );
    assert_eq!(snapshot.stage(AuthStage::CachedBind).succeeded, 2);
    // The outer middleware lookup and both resolver lookups are distinct existing checks.
    assert_eq!(snapshot.stage(AuthStage::SessionLookup).succeeded, 3);
    fixture.close().await;
}

#[tokio::test]
async fn missing_session_cascade_is_recorded_before_authentication_admission() {
    let fixture = Fixture::observed(200, false).await;
    fixture.host.shared.sessions.remove(&fixture.cookie);
    assert_eq!(
        request(&fixture, "fixture").await,
        reqwest::StatusCode::UNAUTHORIZED
    );
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(snapshot.stage(AuthStage::SessionLookup).denied, 1);
    assert_eq!(
        snapshot
            .operation(AuthOperation::ProtectedMiddleware)
            .attempts,
        0
    );
    assert_eq!(
        snapshot
            .operation(AuthOperation::GenericHttpResolver)
            .attempts,
        0
    );
    assert_eq!(
        snapshot.failures[0].operation,
        AuthOperation::ProtectedMiddleware
    );
    fixture.close().await;
}
