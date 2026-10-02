#![cfg(all(feature = "jwt", feature = "introspection"))]
//! Real signatures and loopback HTTP; every credential is synthetic and stays out of output.
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode, get_current_timestamp};
use rom_auth::introspection::{EndpointPolicy, IntrospectionAdapter};
use rom_auth::jwt::{JwtAdapter, TrustedKeys};
use rom_auth::{AuthError, PrincipalKind};

use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};
struct Keys {
    private: Vec<u8>,
    public: Vec<u8>,
}
fn generate() -> Keys {
    let output = Command::new("openssl")
        .args([
            "genpkey",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
        ])
        .stderr(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    let private = output.stdout;
    let mut child = Command::new("openssl")
        .args(["pkey", "-pubout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&private).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    Keys {
        private,
        public: output.stdout,
    }
}
fn keys() -> &'static [Keys; 2] {
    static KEYS: OnceLock<[Keys; 2]> = OnceLock::new();
    KEYS.get_or_init(|| [generate(), generate()])
}
fn public(index: usize) -> DecodingKey {
    DecodingKey::from_rsa_pem(&keys()[index].public).unwrap()
}
fn claims(now: u64) -> Json {
    json!({"iss":"https://jwt.example","aud":["rom-api"],"sub":"same-subject","exp":now+600,"iat":now,"nbf":now,"jti":"synthetic-id","client_id":"browser-client","principal_kind":"human","email":"same@example.invalid","admin":true})
}
fn header(kid: &str) -> Header {
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some(kid.into());
    h.typ = Some("at+jwt".into());
    h
}
fn token(c: &Json, h: Header, index: usize) -> String {
    encode(
        &h,
        c,
        &EncodingKey::from_rsa_pem(&keys()[index].private).unwrap(),
    )
    .unwrap()
}
#[derive(Clone)]
struct Source {
    state: Arc<Mutex<Result<BTreeMap<String, DecodingKey>, AuthError>>>,
    calls: Arc<AtomicUsize>,
}
impl Source {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(Ok(BTreeMap::from([("k1".into(), public(0))])))),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}
impl TrustedKeys for Source {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.state.lock().unwrap().clone()
    }
}
fn jwt(source: Source) -> JwtAdapter<Source> {
    JwtAdapter::configured("authority-jwt", "https://jwt.example", "rom-api", source).unwrap()
}

#[test]
fn jwt_valid_access_profile_constructs_bounded_human_actor_without_credentials_or_claim_dump() {
    let now = get_current_timestamp();
    let t = token(&claims(now), header("k1"), 0);
    let actor = jwt(Source::new()).authenticate(&t, now).unwrap();
    assert_eq!(actor.principal_kind(), PrincipalKind::Human);
    assert_eq!(actor.authority(), "authority-jwt");
    assert_eq!(actor.valid_until(), now + 30);
    let debug = format!("{actor:?}");
    assert!(!debug.contains(&t));
    assert!(!debug.contains("same@example.invalid"));
    assert!(!debug.contains("admin"));
}
#[test]
fn jwt_rejects_wrong_signature_issuer_audience_times_missing_claims_and_profile() {
    let now = get_current_timestamp();
    let mut adapter = jwt(Source::new());
    assert!(
        adapter
            .authenticate(&token(&claims(now), header("k1"), 1), now)
            .is_err()
    );
    for (key, value) in [
        ("iss", json!("https://other.example")),
        ("aud", json!(["other-api"])),
        ("exp", json!(now - 1)),
        ("nbf", json!(now + 60)),
        ("iat", json!(now + 60)),
        ("principal_kind", json!("service")),
        ("cnf", json!({"jkt":"proof-required"})),
        ("sub", json!("")),
    ] {
        let mut c = claims(now);
        c[key] = value;
        assert!(
            adapter
                .authenticate(&token(&c, header("k1"), 0), now)
                .is_err(),
            "rejected {key}"
        );
    }
    for key in [
        "iss",
        "aud",
        "sub",
        "exp",
        "iat",
        "jti",
        "client_id",
        "principal_kind",
    ] {
        let mut c = claims(now);
        c.as_object_mut().unwrap().remove(key);
        assert!(
            adapter
                .authenticate(&token(&c, header("k1"), 0), now)
                .is_err(),
            "required {key}"
        );
    }
    let mut h = header("k1");
    h.typ = Some("JWT".into());
    assert_eq!(
        adapter.authenticate(&token(&claims(now), h, 0), now),
        Err(AuthError::WrongProfile)
    );
    let mut h = Header::new(Algorithm::HS256);
    h.kid = Some("k1".into());
    h.typ = Some("at+jwt".into());
    let forged = encode(
        &h,
        &claims(now),
        &EncodingKey::from_secret(b"synthetic-not-a-rsa-key"),
    )
    .unwrap();
    assert_eq!(
        adapter.authenticate(&forged, now),
        Err(AuthError::WrongProfile)
    );
}
#[test]
fn jwt_untrusted_key_urls_critical_headers_oversize_and_malformed_fail_before_fetch() {
    let now = get_current_timestamp();
    let source = Source::new();
    let mut adapter = jwt(source.clone());
    let mut h = header("k1");
    h.jku = Some("http://127.0.0.1/attacker".into());
    assert_eq!(
        adapter.authenticate(&token(&claims(now), h, 0), now),
        Err(AuthError::WrongProfile)
    );
    let mut h = header("k1");
    h.crit = Some(vec!["unsupported".into()]);
    assert_eq!(
        adapter.authenticate(&token(&claims(now), h, 0), now),
        Err(AuthError::WrongProfile)
    );
    assert_eq!(
        adapter.authenticate(&"x".repeat(16_385), now),
        Err(AuthError::TooLarge)
    );
    assert_eq!(
        adapter.authenticate("not-a-token", now),
        Err(AuthError::Invalid)
    );
    assert_eq!(source.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn jwt_rotation_refresh_is_bounded_replaces_old_keys_and_expired_cache_fails_closed() {
    let now = get_current_timestamp();
    let source = Source::new();
    let mut adapter = jwt(source.clone());
    let old = token(&claims(now), header("k1"), 0);
    let new = token(&claims(now), header("k2"), 1);
    adapter.authenticate(&old, now).unwrap();
    *source.state.lock().unwrap() = Ok(BTreeMap::from([("k2".into(), public(1))]));
    assert_eq!(
        adapter.authenticate(&new, now + 1),
        Err(AuthError::RefreshLimited)
    );
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    adapter.authenticate(&new, now + 5).unwrap();
    assert_eq!(source.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        adapter.authenticate(&old, now + 6),
        Err(AuthError::RefreshLimited)
    );
    assert_eq!(
        adapter.authenticate(&old, now + 10),
        Err(AuthError::UnknownKey)
    );
    *source.state.lock().unwrap() = Err(AuthError::Unavailable);
    assert_eq!(
        adapter.authenticate(&new, now + 41),
        Err(AuthError::Unavailable)
    );
}
#[test]
fn jwt_same_subject_different_issuers_is_not_same_identity() {
    let now = get_current_timestamp();
    let a = jwt(Source::new())
        .authenticate(&token(&claims(now), header("k1"), 0), now)
        .unwrap();
    let mut c = claims(now);
    c["iss"] = json!("https://second.example");
    let mut second = JwtAdapter::configured(
        "authority-second",
        "https://second.example",
        "rom-api",
        Source::new(),
    )
    .unwrap();
    let b = second
        .authenticate(&token(&c, header("k1"), 0), now)
        .unwrap();
    assert_eq!(a.subject(), b.subject());
    assert_ne!(a.authority(), b.authority());
}

struct Reply {
    status: u16,
    body: String,
    delay: Duration,
}
impl Reply {
    fn json(value: Json) -> Self {
        Self {
            status: 200,
            body: value.to_string(),
            delay: Duration::ZERO,
        }
    }
}
struct Fixture {
    url: String,
    stop: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
    valid_request: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new(replies: Vec<Reply>) -> Self {
        Self::with_authorization(
            replies,
            "authorization: Basic Zml4dHVyZS1jbGllbnQ6Zml4dHVyZS1zZWNyZXQ=".into(),
        )
    }
    fn with_authorization(replies: Vec<Reply>, expected_authorization: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/introspect", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicUsize::new(0));
        let valid_request = Arc::new(AtomicBool::new(true));
        let done = stop.clone();
        let count = calls.clone();
        let valid = valid_request.clone();
        let worker = thread::spawn(move || {
            let mut replies: VecDeque<_> = replies.into();
            while !done.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(x) => x,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(_) => break,
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut chunk = [0u8; 1024];
                let header_end = loop {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break None;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break Some(i + 4);
                    }
                    if bytes.len() > 16384 {
                        break None;
                    }
                };
                let Some(header_end) = header_end else {
                    continue;
                };
                let original_headers = String::from_utf8_lossy(&bytes[..header_end]);
                let authenticated = original_headers
                    .lines()
                    .any(|line| line == expected_authorization);
                let headers = original_headers.to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < header_end + length {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                }
                let body = String::from_utf8_lossy(&bytes[header_end..]);
                valid.fetch_and(
                    headers.starts_with("post /introspect ")
                        && authenticated
                        && headers.contains("application/x-www-form-urlencoded")
                        && body.contains("token=")
                        && body.contains("token_type_hint=access_token"),
                    Ordering::SeqCst,
                );
                count.fetch_add(1, Ordering::SeqCst);
                let mut reply = replies.pop_front().unwrap_or(Reply {
                    status: 503,
                    body: "{}".into(),
                    delay: Duration::ZERO,
                });
                if !authenticated {
                    reply.status = 401;
                    reply.body = "{}".into();
                }
                thread::sleep(reply.delay);
                let redirect = if reply.status == 302 {
                    "Location: http://127.0.0.1:9/not-followed\r\n"
                } else {
                    ""
                };
                let response = format!(
                    "HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
                    reply.status,
                    reply.body.len(),
                    redirect,
                    reply.body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            url,
            stop,
            calls,
            valid_request,
            thread: Some(worker),
        }
    }
    fn adapter(&self) -> IntrospectionAdapter {
        IntrospectionAdapter::configured(
            "authority-introspection",
            "https://opaque.example",
            "rom-api",
            &self.url,
            "fixture-client",
            "fixture-secret",
            EndpointPolicy::LoopbackTestOnly,
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn introspection(now: u64) -> Json {
    json!({"active":true,"iss":"https://opaque.example","aud":["rom-api"],"sub":"same-subject","client_id":"same-subject","principal_kind":"service","token_type":"Bearer","exp":now+600,"iat":now,"email":"same@example.invalid"})
}
#[test]
fn introspection_real_http_form_authenticated_service_mapping_and_cache() {
    let now = get_current_timestamp();
    let fixture = Fixture::new(vec![Reply::json(introspection(now))]);
    let mut adapter = fixture.adapter();
    let actor = adapter.authenticate("synthetic-opaque+token", now).unwrap();
    assert_eq!(actor.authority(), "authority-introspection");
    assert_eq!(actor.principal_kind(), PrincipalKind::Service);
    assert_eq!(actor.valid_until(), now + 5);
    assert_eq!(
        adapter
            .authenticate("synthetic-opaque+token", now + 4)
            .unwrap(),
        actor
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    assert!(fixture.valid_request.load(Ordering::SeqCst));
    let debug = format!("{actor:?}");
    assert!(!debug.contains("synthetic-opaque"));
    assert!(!debug.contains("same@example.invalid"));
}
#[test]
fn introspection_inactive_wrong_binding_missing_claims_and_ambiguous_kind_fail_closed() {
    let now = get_current_timestamp();
    for (key, value, expected) in [
        ("active", json!(false), AuthError::Inactive),
        ("iss", json!("https://evil"), AuthError::Binding),
        ("aud", json!(["other"]), AuthError::Binding),
        ("exp", json!(now), AuthError::Expired),
        ("nbf", json!(now + 1), AuthError::Expired),
        ("principal_kind", json!("human"), AuthError::WrongProfile),
        (
            "client_id",
            json!("different-client"),
            AuthError::WrongProfile,
        ),
        ("cnf", json!({"jkt":"proof"}), AuthError::WrongProfile),
        ("token_type", json!("DPoP"), AuthError::WrongProfile),
    ] {
        let mut response = introspection(now);
        response[key] = value;
        let fixture = Fixture::new(vec![Reply::json(response)]);
        assert_eq!(
            fixture.adapter().authenticate("opaque", now),
            Err(expected),
            "rejected {key}"
        );
    }
    for key in [
        "active",
        "iss",
        "aud",
        "sub",
        "exp",
        "client_id",
        "principal_kind",
        "token_type",
    ] {
        let mut response = introspection(now);
        response.as_object_mut().unwrap().remove(key);
        let fixture = Fixture::new(vec![Reply::json(response)]);
        assert!(
            fixture.adapter().authenticate("opaque", now).is_err(),
            "required {key}"
        );
    }
}
#[test]
fn introspection_cache_never_outlives_expiry_and_unavailable_cannot_extend_actor() {
    let now = get_current_timestamp();
    let mut response = introspection(now);
    response["exp"] = json!(now + 2);
    let fixture = Fixture::new(vec![Reply::json(response)]);
    let mut adapter = fixture.adapter();
    assert_eq!(
        adapter.authenticate("opaque", now).unwrap().valid_until(),
        now + 2
    );
    assert!(adapter.authenticate("opaque", now + 1).is_ok());
    assert_eq!(
        adapter.authenticate("opaque", now + 2),
        Err(AuthError::Unavailable)
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 2);
}
#[test]
fn introspection_revocation_observed_after_cache_bound_or_explicit_invalidation() {
    let now = get_current_timestamp();
    let fixture = Fixture::new(vec![
        Reply::json(introspection(now)),
        Reply::json(json!({"active":false})),
    ]);
    let mut adapter = fixture.adapter();
    adapter.authenticate("opaque", now).unwrap();
    assert!(adapter.authenticate("opaque", now + 4).is_ok());
    assert_eq!(
        adapter.authenticate("opaque", now + 5),
        Err(AuthError::Inactive)
    );
    let fixture = Fixture::new(vec![
        Reply::json(introspection(now)),
        Reply::json(json!({"active":false})),
    ]);
    let mut adapter = fixture.adapter();
    adapter.authenticate("opaque", now).unwrap();
    adapter.invalidate();
    assert_eq!(
        adapter.authenticate("opaque", now + 1),
        Err(AuthError::Inactive)
    );
}
#[test]
fn introspection_redirect_timeout_bad_json_oversize_and_unsafe_endpoint_rejected() {
    let now = get_current_timestamp();
    for (reply, expected) in [
        (
            Reply {
                status: 302,
                body: "{}".into(),
                delay: Duration::ZERO,
            },
            AuthError::Unavailable,
        ),
        (
            Reply {
                status: 200,
                body: "invalid".into(),
                delay: Duration::ZERO,
            },
            AuthError::Invalid,
        ),
        (
            Reply {
                status: 200,
                body: "x".repeat(16_385),
                delay: Duration::ZERO,
            },
            AuthError::TooLarge,
        ),
        (
            Reply {
                status: 200,
                body: introspection(now).to_string(),
                delay: Duration::from_millis(650),
            },
            AuthError::Unavailable,
        ),
    ] {
        let fixture = Fixture::new(vec![reply]);
        assert_eq!(fixture.adapter().authenticate("opaque", now), Err(expected));
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    }
    for url in [
        "http://evil.example/introspect",
        "http://localhost/introspect",
        "file:///private",
        "https://user:pass@example.com/introspect",
    ] {
        assert!(matches!(
            IntrospectionAdapter::configured(
                "a",
                "https://i",
                "r",
                url,
                "c",
                "s",
                EndpointPolicy::LoopbackTestOnly
            ),
            Err(AuthError::UnsafeEndpoint)
        ));
    }
}
#[test]
fn introspection_cache_capacity_is_bounded_and_oversize_token_does_not_send() {
    let now = get_current_timestamp();
    let fixture = Fixture::new((0..9).map(|_| Reply::json(introspection(now))).collect());
    let mut adapter = fixture.adapter();
    assert_eq!(
        adapter.authenticate(&"x".repeat(4097), now),
        Err(AuthError::TooLarge)
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    for i in 0..9 {
        adapter.authenticate(&format!("opaque-{i}"), now).unwrap();
        assert!(adapter.cache_entries() <= 8);
    }
}

#[test]
fn introspection_server_rejects_wrong_client_credentials() {
    let now = get_current_timestamp();
    let fixture = Fixture::new(vec![Reply::json(introspection(now))]);
    let mut adapter = IntrospectionAdapter::configured(
        "authority-introspection",
        "https://opaque.example",
        "rom-api",
        &fixture.url,
        "fixture-client",
        "wrong-fixture-secret",
        EndpointPolicy::LoopbackTestOnly,
    )
    .unwrap();
    assert_eq!(
        adapter.authenticate("opaque", now),
        Err(AuthError::Unavailable)
    );
    assert!(!fixture.valid_request.load(Ordering::SeqCst));
}

#[test]
fn host_configuration_requires_explicit_bindings_and_transport_policy() {
    assert!(matches!(
        JwtAdapter::configured("", "https://jwt.example", "rom-api", Source::new()),
        Err(AuthError::InvalidConfiguration)
    ));
    let fixture = Fixture::new(vec![]);
    assert!(matches!(
        IntrospectionAdapter::configured(
            "authority",
            "https://issuer.example",
            "rom-api",
            &fixture.url,
            "fixture-client",
            "fixture-secret",
            EndpointPolicy::HttpsOnly
        ),
        Err(AuthError::UnsafeEndpoint)
    ));
    assert!(
        IntrospectionAdapter::configured(
            "authority",
            "https://issuer.example",
            "rom-api",
            "https://issuer.example/introspect",
            "fixture-client",
            "fixture-secret",
            EndpointPolicy::HttpsOnly
        )
        .is_ok()
    );
    assert!(matches!(
        IntrospectionAdapter::configured(
            "authority",
            "https://issuer.example",
            "rom-api",
            "https://issuer.example/introspect",
            "fixture-client",
            "fixture-secret",
            EndpointPolicy::LoopbackTestOnly
        ),
        Err(AuthError::UnsafeEndpoint)
    ));
}
#[test]
fn jwt_uses_one_trusted_host_clock_and_enforces_exact_expiry() {
    let now = 1_000;
    let t = token(&claims(now), header("k1"), 0);
    let mut verifier = jwt(Source::new());
    let proof = verifier.authenticate(&t, now).unwrap();
    assert_eq!(proof.subject(), "same-subject");
    assert_eq!(proof.valid_until(), now + 30);
    assert!(!format!("{proof:?}").contains(proof.subject()));
    assert_eq!(
        verifier.authenticate(&t, now + 600),
        Err(AuthError::Expired)
    );
}
#[test]
fn bounded_key_source_rejects_oversized_set_and_missing_requested_key() {
    let now = get_current_timestamp();
    let source = Source::new();
    *source.state.lock().unwrap() = Ok((0..9).map(|i| (format!("k{i}"), public(0))).collect());
    assert_eq!(
        jwt(source).authenticate(&token(&claims(now), header("k1"), 0), now),
        Err(AuthError::Invalid)
    );
    assert_eq!(
        jwt(Source::new()).authenticate(&token(&claims(now), header("missing"), 0), now),
        Err(AuthError::UnknownKey)
    );
}

#[test]
fn introspection_basic_credentials_use_oauth_form_encoding() {
    let now = get_current_timestamp();
    let fixture = Fixture::with_authorization(
        vec![Reply::json(introspection(now))],
        "authorization: Basic Y2xpZW50K2lkJTJCOnNlY3JldCslM0ElMkIlMjY=".into(),
    );
    let mut adapter = IntrospectionAdapter::configured(
        "authority-introspection",
        "https://opaque.example",
        "rom-api",
        &fixture.url,
        "client id+",
        "secret :+&",
        EndpointPolicy::LoopbackTestOnly,
    )
    .unwrap();
    assert!(adapter.authenticate("opaque", now).is_ok());
    assert!(fixture.valid_request.load(Ordering::SeqCst));
}
