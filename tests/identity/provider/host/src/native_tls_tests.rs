use super::{Configuration, TransportProfile, approved_origin, transport_profile};

fn configuration() -> Configuration {
    Configuration {
        adapter: "sqlite".into(),
        assets_directory: "/root/ROM/tests/identity/provider/host/assets".into(),
        database: "/var/tmp/rom-010-authentik-20261007/run/volume/private/native-tls-test/database".into(),
        stop_file: "/var/tmp/rom-010-authentik-20261007/run/volume/private/native-tls-test/stop".into(),
        control_directory: None,
        public_origin: Some("https://127.0.0.1:44389".into()),
        private_token_endpoint: None,
        private_jwks_endpoint: None,
        issuer: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/".into(),
        client_id: "synthetic".into(),
        client_secret: "synthetic-not-used".into(),
        authorization_endpoint: "https://127.0.0.1:44392/application/o/authorize/".into(),
        token_endpoint: "https://127.0.0.1:44392/application/o/token/".into(),
        jwks_endpoint: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/jwks/".into(),
        verified_synthetic_subject: "synthetic".into(),
    }
}

#[test]
fn native_tls_is_a_distinct_closed_profile_without_a_backchannel() {
    let configuration = configuration();
    assert_eq!(transport_profile(&configuration).unwrap(), TransportProfile::NativeTls);
    assert_eq!(approved_origin(&configuration).unwrap(), "https://127.0.0.1:44389");
}

#[test]
fn terminated_loopback_remains_explicit_and_mixed_routes_are_rejected() {
    let mut configuration = configuration();
    configuration.private_token_endpoint = Some("http://127.0.0.1:44393/application/o/token/".into());
    assert!(approved_origin(&configuration).is_err());
    configuration.private_jwks_endpoint = Some("http://127.0.0.1:44393/application/o/rom-synthetic-identity/jwks/".into());
    assert_eq!(transport_profile(&configuration).unwrap(), TransportProfile::TerminatedLoopback);
    configuration.private_jwks_endpoint = Some("https://127.0.0.1:44393/application/o/rom-synthetic-identity/jwks/".into());
    assert!(approved_origin(&configuration).is_err());
}

#[test]
fn native_tls_rejects_http_acquisition_or_a_different_issuer_origin() {
    for field in ["token", "jwks", "issuer"] {
        let mut configuration = configuration();
        match field {
            "token" => configuration.token_endpoint = "http://127.0.0.1:44390/application/o/token/".into(),
            "jwks" => configuration.jwks_endpoint = "https://wrong.invalid/application/o/rom-synthetic-identity/jwks/".into(),
            _ => configuration.issuer = "https://wrong.invalid/application/o/rom-synthetic-identity/".into(),
        }
        assert!(approved_origin(&configuration).is_err());
    }
}

#[test]
fn original_http_fixture_still_has_no_endpoint_override() {
    let mut configuration = configuration();
    configuration.public_origin = None;
    configuration.issuer = configuration.issuer.replace("https://127.0.0.1:44392", "http://127.0.0.1:44390");
    configuration.authorization_endpoint = configuration.authorization_endpoint.replace("https://127.0.0.1:44392", "http://127.0.0.1:44390");
    configuration.token_endpoint = configuration.token_endpoint.replace("https://127.0.0.1:44392", "http://127.0.0.1:44390");
    configuration.jwks_endpoint = configuration.jwks_endpoint.replace("https://127.0.0.1:44392", "http://127.0.0.1:44390");
    assert_eq!(transport_profile(&configuration).unwrap(), TransportProfile::HttpLoopback);
    assert_eq!(approved_origin(&configuration).unwrap(), "http://127.0.0.1:44391");
    configuration.private_token_endpoint = Some("http://127.0.0.1:44393/application/o/token/".into());
    assert!(approved_origin(&configuration).is_err());
}
