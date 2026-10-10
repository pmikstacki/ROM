use super::oidc::{original_expiry, parse_jwks};

#[tokio::test]
async fn client_secret_basic_uses_form_encoding_in_actual_network_header() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let received = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = vec![0; 4096];
        let n = socket.read(&mut bytes).await.unwrap();
        let header = String::from_utf8(bytes[..n].to_vec()).unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        header
    });
    let request = reqwest::Client::new().post(format!("http://{address}/token"));
    super::oidc::client_secret_basic(request, "studio: client+", "secret:% +")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .unwrap();
    let header = tokio::time::timeout(std::time::Duration::from_secs(2), received)
        .await
        .unwrap()
        .unwrap();
    let authorization = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(": ")?;
            name.eq_ignore_ascii_case("authorization").then_some(value)
        })
        .unwrap();
    assert_eq!(
        authorization,
        "Basic c3R1ZGlvJTNBK2NsaWVudCUyQjpzZWNyZXQlM0ElMjUrJTJC"
    );
}

#[test]
fn key_sets_reject_duplicate_ids_and_token_selected_trust_fields() {
    let duplicate = br#"{"keys":[{"kid":"same","kty":"RSA","alg":"RS256","use":"sig","n":"AQAB","e":"AQAB"},{"kid":"same","kty":"RSA","alg":"RS256","use":"sig","n":"AQAB","e":"AQAB"}]}"#;
    assert!(parse_jwks(duplicate).is_err());
    for wrong in [
        br#"{"keys":[{"kid":"k","kty":"oct","k":"secret"}]}"#.as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","alg":"RS256","use":"enc","n":"AQAB","e":"AQAB"}]}"#
            .as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","alg":"HS256","n":"AQAB","e":"AQAB"}]}"#.as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","key_ops":["sign"],"n":"AQAB","e":"AQAB"}]}"#
            .as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","jku":"https://evil.example","n":"AQAB","e":"AQAB"}]}"#
            .as_slice(),
        br#"{"keys":[]}"#.as_slice(),
    ] {
        assert!(parse_jwks(wrong).is_err());
    }
}

#[test]
fn original_expiry_parser_is_a_bound_not_an_authenticator() {
    // This intentionally unsigned value cannot establish a session. The caller must verify first.
    let token = "e30.eyJleHAiOjIwMDB9.unsigned";
    assert_eq!(original_expiry(token).unwrap(), 2000);
    for invalid in [
        "bad",
        "e30.e30.x",
        "e30.eyJleHAiOi0xfQ.x",
        "e30.eyJleHAiOjIuNX0.x",
    ] {
        assert!(original_expiry(invalid).is_err());
    }
}

async fn original_acquisition_response(status: u16, body: &'static [u8]) -> rom::Result<Vec<u8>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket
            .write_all(
                format!(
                    "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        socket.write_all(body).await.unwrap();
    });
    let result = super::oidc::bounded(
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}/jwks"))
            .timeout(std::time::Duration::from_secs(2)),
        65_536,
    )
    .await;
    server.await.unwrap();
    result
}

#[tokio::test]
async fn jwks_acquisition_separates_temporary_provider_status_from_credential_rejection() {
    for status in [429, 500, 502, 503, 504] {
        assert!(
            matches!(
                original_acquisition_response(status, b"temporary provider failure").await,
                Err(rom::Error::Overloaded)
            ),
            "status {status}"
        );
    }
    for status in [301, 400, 401, 403, 404] {
        assert!(
            matches!(
                original_acquisition_response(status, b"rejected").await,
                Err(rom::Error::Denied)
            ),
            "status {status}"
        );
    }
    let malformed = original_acquisition_response(200, b"malformed provider keys")
        .await
        .unwrap();
    assert!(matches!(
        super::oidc::parse_jwks(&malformed),
        Err(rom::Error::Denied)
    ));
}

#[tokio::test]
async fn jwks_acquisition_keeps_network_failure_temporary_without_admitting_tls_protocol_errors() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let refused = listener.local_addr().unwrap();
    drop(listener);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert!(matches!(
        super::oidc::bounded(
            client
                .get(format!("http://{refused}/jwks"))
                .timeout(std::time::Duration::from_secs(2)),
            65_536
        )
        .await,
        Err(rom::Error::Overloaded)
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await
            .unwrap();
    });
    assert!(matches!(
        super::oidc::bounded(
            client
                .get(format!("https://{address}/jwks"))
                .timeout(std::time::Duration::from_secs(2)),
            65_536
        )
        .await,
        Err(rom::Error::Denied)
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn jwks_acquisition_timeout_does_not_destroy_or_extend_original_session_expiry() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });
    let result = super::oidc::bounded(
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}/jwks"))
            .timeout(std::time::Duration::from_millis(100)),
        65_536,
    )
    .await;
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    let error = result.unwrap_err();
    assert!(matches!(error, rom::Error::Overloaded));
    let store = super::session::SessionStore::new(1, 600);
    let session = store
        .insert(
            super::session::SessionEvidence {
                actor: rom::Actor::trusted("fixture", "alice").expires_at(150),
                user_id: "alice-user".into(),
                token_expiry: 150,
                credentials: None,
            },
            100,
        )
        .unwrap();
    store.failed(session.cookie(), &error);
    assert!(store.lookup(session.cookie(), 149).is_some());
    assert_eq!(session.expires_at(), 150);
    assert!(store.lookup(session.cookie(), 150).is_none());
    assert!(*session.cancellation().borrow());
}
