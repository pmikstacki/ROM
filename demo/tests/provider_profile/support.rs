use rom::{Clock, Storage};
use rom_demo::provider_profile::Provisioning;
use rom_identity::{IdentityProvider, ProviderProfile, User};
use std::sync::Arc;
#[path = "../provider_auth/support.rs"]
#[allow(dead_code, unused_imports)]
mod auth_support;
pub use auth_support::child_process::Process;
pub use auth_support::{Provider, Scratch, headers};
pub struct Time;
impl Clock for Time {
    fn now(&self) -> u64 {
        100
    }
}
pub fn settings(endpoint: &str) -> Provisioning {
    Provisioning {
        provider_id: "provider".into(),
        user_id: "user".into(),
        service_subject: "service".into(),
        provider: IdentityProvider {
            enabled: true,
            profile: ProviderProfile::OAuthIntrospectionService,
            issuer: "https://fixture.invalid".into(),
            audience: "rom-api".into(),
            endpoint: Some(endpoint.into()),
            credential_ref: Some("secret-v1".into()),
        },
        user: User {
            enabled: true,
            display_name: "Fixture".into(),
        },
    }
}
pub fn storage(redb: bool, path: &std::path::Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
pub enum Db {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Db {
    pub fn open(redb: bool, path: &std::path::Path) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    pub fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(db) => db.clone(),
            Self::Redb(db) => db.clone(),
        }
    }
    pub fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(db) => db.counts().unwrap(),
            Self::Redb(db) => db.counts().unwrap(),
        }
    }
    pub fn archive(&self, path: &std::path::Path) -> rom_backup::Snapshot {
        let limits = rom_backup::BackupLimits::default();
        let backend = match self {
            Self::Sqlite(db) => {
                db.backup_to(path, limits).unwrap();
                rom_backup::Backend::Sqlite
            }
            Self::Redb(db) => {
                db.backup_to(path, limits).unwrap();
                rom_backup::Backend::Redb
            }
        };
        rom_backup::read(path, backend, limits).unwrap().1
    }
}
pub struct LostAck {
    pub base: Arc<dyn Storage>,
    pub remaining: std::sync::atomic::AtomicUsize,
}
impl Storage for LostAck {
    fn acquire_owner(&self) -> rom::Result<rom::StorageOwner> {
        self.base.acquire_owner()
    }
    fn register(&self, d: &[rom::Descriptor]) -> rom::Result<()> {
        self.base.register(d)
    }
    fn capabilities(&self) -> rom::Capabilities {
        self.base.capabilities()
    }
    fn retry_epochs(&self) -> rom::Result<rom::RetryEpochs> {
        self.base.retry_epochs()
    }
    fn load(&self, k: &rom::Key) -> rom::Result<Option<rom::Row>> {
        self.base.load(k)
    }
    fn snapshot(&self, k: &str, n: usize, b: usize) -> rom::Result<Vec<rom::Row>> {
        self.base.snapshot(k, n, b)
    }
    fn receipt(&self, i: &str) -> rom::Result<Option<rom::Receipt>> {
        self.base.receipt(i)
    }
    fn commit(&self, b: &rom::Bundle) -> rom::Result<rom::Receipt> {
        let result = self.base.commit(b)?;
        if self
            .remaining
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
            == 1
        {
            Err(rom::Error::Unknown)
        } else {
            Ok(result)
        }
    }
}
pub fn authentication(
    runtime: &rom::Runtime,
    config: &Provisioning,
    scratch: &Scratch,
    limits: rom_demo::provider_profile::AuthLimits,
    material: &[u8],
) -> rom_demo::provider_profile::HostAuth {
    use rom_demo::provider_profile::{ApprovedProvider, HostAuth, SecretFiles};
    HostAuth::new(
        runtime.clone(),
        rom_demo::provider_profile::configuration_reader(),
        Arc::new(Time),
        ApprovedProvider {
            authority: config.provider_id.clone(),
            issuer: config.provider.issuer.clone(),
            audience: config.provider.audience.clone(),
            endpoint: config.provider.endpoint.clone().unwrap(),
            introspection_client: "introspector".into(),
        },
        SecretFiles::new(std::collections::BTreeMap::from([(
            "secret-v1".into(),
            scratch.file("secret", material, 0o600),
        )]))
        .unwrap(),
        rom_auth::introspection::EndpointPolicy::LoopbackTestOnly,
        limits,
    )
    .unwrap()
}
