//! Negative admission tests for trusted external HTTPS profiles.
use crate::studio_profile::TrustedProfile;
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
fn fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rom-studio-profile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir(&path).expect("fixture directory");
    path
}
fn config(path: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "public_origin":"https://studio.example", "issuer":"https://identity.example",
        "authorization_endpoint":"https://identity.example/auth", "token_endpoint":"https://identity.example/token", "jwks_endpoint":"https://identity.example/jwks", "client_secret_file":path.to_str().expect("path")
    })
}
#[test]
fn trusted_profile_uses_private_external_secret_without_enabling_public_http() {
    let root = fixture();
    let secret = root.join("secret");
    let profile = root.join("profile.json");
    std::fs::write(&secret, "private-value\n").expect("secret");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).expect("mode");
    std::fs::write(&profile, config(&secret).to_string()).expect("profile");
    let loaded = TrustedProfile::read(&profile).expect("profile admitted");
    assert_eq!(loaded.origin, "https://studio.example");
    assert_eq!(loaded.provider.authority, "local");
    assert_eq!(
        loaded.provider.client_secret.as_deref(),
        Some("private-value")
    );
    let mut bad = config(&secret);
    bad["public_origin"] = "http://studio.example".into();
    std::fs::write(&profile, bad.to_string()).expect("profile");
    assert!(TrustedProfile::read(&profile).is_err());
    std::fs::write(&profile, config(&secret).to_string()).expect("profile");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644)).expect("mode");
    assert!(TrustedProfile::read(&profile).is_err());
    std::fs::remove_dir_all(root).expect("cleanup private synthetic fixture");
}
#[test]
fn trusted_profile_rejects_symlink_secret_unknown_field_and_partial_backchannel() {
    let root = fixture();
    let secret = root.join("secret");
    let alias = root.join("alias");
    let profile = root.join("profile.json");
    std::fs::write(&secret, "private-value").expect("secret");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).expect("mode");
    std::os::unix::fs::symlink(&secret, &alias).expect("symlink");
    std::fs::write(&profile, config(&alias).to_string()).expect("profile");
    assert!(TrustedProfile::read(&profile).is_err());
    let mut bad = config(&secret);
    bad["secret"] = "must not be inline".into();
    std::fs::write(&profile, bad.to_string()).expect("profile");
    assert!(TrustedProfile::read(&profile).is_err());
    let mut bad = config(&secret);
    bad["backchannel_token_endpoint"] = "http://127.0.0.1:1234/token".into();
    std::fs::write(&profile, bad.to_string()).expect("profile");
    assert!(TrustedProfile::read(&profile).is_err());
    std::fs::remove_dir_all(root).expect("cleanup private synthetic fixture");
}
#[test]
fn trusted_profile_limits_credential_reads_and_redacts_invalid_utf8() {
    let root = fixture();
    let secret = root.join("secret");
    let profile = root.join("profile.json");
    std::fs::write(&secret, [0xff, 0xfe]).expect("synthetic invalid credential");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).expect("mode");
    std::fs::write(&profile, config(&secret).to_string()).expect("profile");
    let error = TrustedProfile::read(&profile)
        .err()
        .expect("invalid credential rejected");
    assert_eq!(error.to_string(), "profile file must use UTF-8");
    assert!(!format!("{error:?}").contains("255"));
    std::fs::write(&secret, vec![b'x'; 4097]).expect("synthetic oversized credential");
    assert!(TrustedProfile::read(&profile).is_err());
    std::fs::remove_dir_all(root).expect("cleanup private synthetic fixture");
}
