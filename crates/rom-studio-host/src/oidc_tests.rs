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
