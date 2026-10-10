use rom_ai::{AiError, AiFuture};
use rom_openrouter::{CredentialSource, OpenRouter, OpenRouterConfig, SecretToken};
use std::sync::Arc;

struct Credentials;
impl CredentialSource for Credentials {
    fn resolve<'a>(&'a self, _: &'a str) -> AiFuture<'a, SecretToken> {
        Box::pin(async { SecretToken::new("fixture-secret") })
    }
}

fn config(endpoint: &str) -> OpenRouterConfig {
    OpenRouterConfig {
        endpoint: endpoint.into(),
        credential_ref: "fixture-reference".into(),
        catalog_ttl_seconds: 60,
        response_bytes: 128 * 1024,
    }
}

#[test]
fn production_origin_rejects_credential_and_route_injection() {
    for endpoint in [
        "http://openrouter.ai/api/v1",
        "https://attacker.invalid/api/v1",
        "https://secret@openrouter.ai/api/v1",
        "https://openrouter.ai/api/v1?redirect=private",
        "https://openrouter.ai/api/v1#private",
        "https://openrouter.ai/api/v1/../../private",
        "https://openrouter.ai:444/api/v1",
    ] {
        assert!(matches!(
            OpenRouter::new(config(endpoint), Arc::new(Credentials)),
            Err(AiError::InvalidRequest)
        ));
    }
}

#[test]
fn credentials_are_bounded_valid_headers_and_debug_is_redacted() {
    for token in [
        String::new(),
        "key\r\nInjected: yes".into(),
        "x".repeat(4097),
    ] {
        assert!(matches!(
            SecretToken::new(token),
            Err(AiError::InvalidRequest)
        ));
    }
    let secret = SecretToken::new("planted-secret-marker").unwrap();
    assert!(!format!("{secret:?}").contains("planted-secret-marker"));
}

#[test]
fn host_limits_are_positive_finite() {
    for bytes in [0, 1024 * 1024 + 1] {
        let mut config = config("https://openrouter.ai/api/v1");
        config.response_bytes = bytes;
        assert!(matches!(
            OpenRouter::new(config, Arc::new(Credentials)),
            Err(AiError::InvalidRequest)
        ));
    }
}
