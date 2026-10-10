//! Actual verified transport rejects the valid self-signed public test certificate.
#![cfg(feature = "test-support")]
#[path = "support/credentials.rs"]
mod credentials;
use rom_ai::{AiError, Deadline, Provider};
use rom_openrouter::{OpenRouter, OpenRouterConfig};
use rustls::{
    ServerConfig, ServerConnection, StreamOwned,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};
use std::{
    io::Read,
    net::TcpListener,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn untrusted_signed_certificate_never_receives_http_credentials() {
    let certificate =
        CertificateDer::from(include_bytes!("fixtures/loopback-certificate.der").to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        include_bytes!("fixtures/loopback-key.der").to_vec(),
    ));
    let config = ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![certificate], key)
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("https://{}/api/v1", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(2);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until =>
                {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(error) => panic!("TLS fixture accept: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut tls = StreamOwned::new(ServerConnection::new(Arc::new(config)).unwrap(), stream);
        let mut plaintext = [0_u8; 1024];
        let error = tls
            .read(&mut plaintext)
            .expect_err("untrusted certificate must fail before HTTP");
        assert!(plaintext.iter().all(|byte| *byte == 0));
        assert!(matches!(
            error
                .get_ref()
                .and_then(|error| error.downcast_ref::<rustls::Error>()),
            Some(rustls::Error::AlertReceived(
                rustls::AlertDescription::UnknownCA | rustls::AlertDescription::CertificateUnknown
            ))
        ));
    });
    let provider = OpenRouter::for_loopback(
        OpenRouterConfig {
            endpoint,
            credential_ref: "fixture-reference".into(),
            catalog_ttl_seconds: 60,
            response_bytes: 1024,
        },
        Arc::new(credentials::Credentials),
    )
    .unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert_eq!(
        provider
            .catalog(Deadline::remaining(now, now, now + 1000).unwrap())
            .await,
        Err(AiError::UnknownOutcome)
    );
    server.join().unwrap();
}
