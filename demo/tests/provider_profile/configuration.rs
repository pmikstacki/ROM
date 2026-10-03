use super::support::Scratch;
use rom_demo::provider_profile::ProfileConfig;
#[test]
fn config_rejects_duplicate_unknown_fields_and_nonprivate_file_without_diagnostics() {
    let scratch = Scratch::new();
    for (name, bytes, mode) in [
        (
            "duplicate",
            b"{\"version\":1,\"version\":1}".as_slice(),
            0o600,
        ),
        (
            "unknown",
            b"{\"version\":1,\"secret-marker\":true}".as_slice(),
            0o600,
        ),
        ("public", b"{}".as_slice(), 0o644),
    ] {
        let error = match ProfileConfig::load(&scratch.file(name, bytes, mode)) {
            Ok(_) => panic!("invalid config accepted"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains("secret-marker"));
    }
}
pub fn document(endpoint: &str) -> rom::Value {
    rom::json!({"version":1,"provider":{"authority":"provider","issuer":"https://fixture.invalid","audience":"rom-api","endpoint":endpoint,"introspection_client":"introspector","credential_ref":"secret-v1"},"user":{"id":"user","display_name":"Fixture"},"service_subject":"service","secrets":{"secret-v1":"private/v1","secret-v2":"private/v2"}})
}
#[test]
fn valid_config_safe_defaults_and_closed_bounded_schema() {
    use rom_auth::introspection::EndpointPolicy;
    let scratch = Scratch::new();
    let value = document("https://fixture.invalid/introspect");
    let loaded = ProfileConfig::load(&scratch.file(
        "valid",
        serde_json::to_string(&value).unwrap().as_bytes(),
        0o600,
    ))
    .unwrap();
    assert!(matches!(loaded.endpoint_policy, EndpointPolicy::HttpsOnly));
    assert_eq!(loaded.auth.jobs, 4);
    assert_eq!(
        loaded.auth.response_timeout,
        std::time::Duration::from_secs(2)
    );
    assert_eq!(loaded.provisioning.service_subject, "service");
    let mut cases = Vec::new();
    let mut v = value.clone();
    v["version"] = rom::json!(2);
    cases.push(v);
    let mut v = value.clone();
    v["provider"]["extra"] = rom::json!(true);
    cases.push(v);
    let mut v = value.clone();
    v["user"]["id"] = rom::json!("");
    cases.push(v);
    let mut v = value.clone();
    v["auth"] = rom::json!({"jobs":0,"response_timeout_ms":1});
    cases.push(v);
    let mut v = value.clone();
    v["endpoint_policy"] = rom::json!("anything");
    cases.push(v);
    let mut v = value.clone();
    v["provider"]["credential_ref"] = rom::json!("unknown");
    cases.push(v);
    let mut v = value.clone();
    for i in 0..9 {
        v["secrets"][format!("ref{i}")] = rom::json!("private/ref");
    }
    cases.push(v);
    for (i, value) in cases.into_iter().enumerate() {
        assert!(
            ProfileConfig::load(&scratch.file(
                &format!("bad{i}"),
                serde_json::to_string(&value).unwrap().as_bytes(),
                0o600
            ))
            .is_err()
        );
    }
    assert!(ProfileConfig::load(&scratch.file("oversized", &vec![b' '; 65_537], 0o600)).is_err());
    let nested = serde_json::to_string(&value).unwrap().replace(
        "\"authority\":\"provider\"",
        "\"authority\":\"provider\",\"authority\":\"provider\"",
    );
    assert!(ProfileConfig::load(&scratch.file("nested", nested.as_bytes(), 0o600)).is_err());
}
