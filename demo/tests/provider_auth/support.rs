use rom::{Actor, Clock, Command, PrincipalKind, Resource, Runtime};
use rom_auth::introspection::EndpointPolicy;
use rom_demo::provider_profile::{ApprovedProvider, AuthLimits, HostAuth, SecretFiles};
use rom_http::HeaderMap;
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::PathBuf, sync::Arc};

#[allow(dead_code)]
#[path = "../../../tests/persistence/tests/support/child_process.rs"]
pub(crate) mod child_process;

pub const NOW: u64 = 100;
pub struct Time;
impl Clock for Time {
    fn now(&self) -> u64 {
        NOW
    }
}
pub fn host() -> Actor {
    Actor::trusted("host", "auth-fixture")
}

pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        use std::{
            os::unix::fs::DirBuilderExt,
            sync::atomic::{AtomicU64, Ordering},
        };
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..1024 {
            let id = NEXT
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .expect("scratch identity exhausted");
            let path =
                std::env::temp_dir().join(format!("rom-host-auth-{}-{id}", std::process::id()));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("scratch creation failed: {error}"),
            }
        }
        panic!("scratch creation attempts exhausted");
    }
    pub fn file(&self, name: &str, bytes: &[u8], mode: u32) -> PathBuf {
        use std::os::unix::fs::OpenOptionsExt;
        let path = self.0.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&path)
            .unwrap();
        file.write_all(bytes).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[path = "provider.rs"]
mod provider;
pub use provider::Provider;

pub struct Fixture {
    pub runtime: Runtime,
    pub clock: Arc<dyn Clock>,
    pub provider: IdentityProvider,
}
impl Fixture {
    pub async fn new(endpoint: &str) -> Self {
        let clock: Arc<dyn Clock> = Arc::new(Time);
        let gate = IdentityGate::default()
            .allow_host("host", PrincipalKind::Embedded, "auth-fixture")
            .unwrap();
        let runtime = Runtime::builder()
            .clock(clock.clone())
            .actor_gate(Arc::new(gate))
            .resource(
                User::definition()
                    .policy(|a, _, _| a == &host())
                    .allow_all_fields(),
            )
            .resource(
                IdentityProvider::definition()
                    .policy(|a, _, _| a == &host())
                    .allow_all_fields(),
            )
            .resource(
                IdentityLink::definition()
                    .policy(|a, _, _| a == &host())
                    .allow_all_fields(),
            )
            .build(
                Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
                Runtime::shared_cpu_pool(1).unwrap(),
            )
            .unwrap();
        let provider = IdentityProvider {
            enabled: true,
            profile: ProviderProfile::OAuthIntrospectionService,
            issuer: "https://fixture.invalid".into(),
            audience: "rom-api".into(),
            endpoint: Some(endpoint.into()),
            credential_ref: Some("secret-v1".into()),
        };
        runtime
            .execute(
                &host(),
                Command::create("provider", provider.clone()).idempotency("provider"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &host(),
                Command::create(
                    "user",
                    User {
                        enabled: true,
                        display_name: "Fixture".into(),
                    },
                )
                .idempotency("user"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &host(),
                Command::create(
                    &link_key("provider", PrincipalKind::Service, "service"),
                    IdentityLink {
                        enabled: true,
                        authority: "provider".into(),
                        subject: "service".into(),
                        principal_kind: "service".into(),
                        user_id: "user".into(),
                    },
                )
                .idempotency("link"),
            )
            .await
            .unwrap();
        Self {
            runtime,
            clock,
            provider,
        }
    }
    pub fn approved(&self) -> ApprovedProvider {
        ApprovedProvider {
            authority: "provider".into(),
            issuer: self.provider.issuer.clone(),
            audience: self.provider.audience.clone(),
            endpoint: self.provider.endpoint.clone().unwrap(),
            introspection_client: "introspector".into(),
        }
    }
    pub fn auth(&self, path: PathBuf, limits: AuthLimits) -> HostAuth {
        HostAuth::new(
            self.runtime.clone(),
            host(),
            self.clock.clone(),
            self.approved(),
            SecretFiles::new(BTreeMap::from([("secret-v1".into(), path)])).unwrap(),
            EndpointPolicy::LoopbackTestOnly,
            limits,
        )
        .unwrap()
    }
    pub async fn change(&mut self) {
        self.runtime
            .execute(
                &host(),
                Command::replace("provider", self.provider.clone())
                    .at_revision(1)
                    .idempotency("change"),
            )
            .await
            .unwrap();
    }
    pub async fn finish(self) {
        self.runtime.shutdown().await.unwrap();
    }
}
pub fn headers(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", value.parse().unwrap());
    headers
}
pub async fn denied(auth: &HostAuth) {
    assert!(matches!(
        auth.resolver()(headers("Bearer fixture-token")).await,
        Err(rom::Error::Denied)
    ));
}
