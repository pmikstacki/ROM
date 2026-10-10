//! Explicit loopback-only Studio demo host. Test controls use a private Unix socket, never an HTTP endpoint.
use crate::{host_signals::SignalReceiver, smoke::SmokeResult, studio_application, studio_startup};
use rom::{Clock, Storage, SystemClock};
use rom_studio_host::{HostConfig, OidcProviderConfig, StudioHost};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub struct DemoClock {
    offset: AtomicU64,
}
impl DemoClock {
    pub fn advance(&self, seconds: u64) -> rom::Result<()> {
        self.offset
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(seconds)
            })
            .map_err(|_| rom::Error::TooLarge)?;
        Ok(())
    }
}
impl Clock for DemoClock {
    fn now(&self) -> u64 {
        SystemClock
            .now()
            .saturating_add(self.offset.load(Ordering::SeqCst))
    }
}
pub async fn run(
    redb: bool,
    path: &str,
    port: u16,
    assets: &Path,
    issuer: &str,
    controls: Option<&Path>,
) -> SmokeResult<()> {
    run_config(redb, path, port, assets, issuer, controls, None).await
}
pub async fn run_profile(
    redb: bool,
    path: &str,
    port: u16,
    assets: &Path,
    profile: &Path,
) -> SmokeResult<()> {
    let profile = crate::studio_profile::TrustedProfile::read(profile)?;
    let issuer = profile.provider.issuer.clone();
    run_config(redb, path, port, assets, &issuer, None, Some(profile)).await
}
async fn run_config(
    redb: bool,
    path: &str,
    port: u16,
    assets: &Path,
    issuer: &str,
    controls: Option<&Path>,
    profile: Option<crate::studio_profile::TrustedProfile>,
) -> SmokeResult<()> {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path)?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path)?)
    };
    let clock = Arc::new(DemoClock {
        offset: AtomicU64::new(0),
    });
    let runtime = studio_application::build(storage.clone(), clock.clone())?;
    let gate = crate::studio_blobs::PublicationGate::new();
    let blobs = crate::studio_blobs::build(runtime.clone(), Path::new(path), gate.clone())?;
    let result = async {
        studio_startup::seed_all(&runtime,storage.as_ref(),issuer).await?;
        let listener=tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,port)).await?;
        let origin=profile.as_ref().map(|value|value.origin.clone()).unwrap_or(format!("http://{}",listener.local_addr()?));
        let mut config=HostConfig::new(&origin,"/rom-studio/",assets,studio_application::host_actor())
            .allow_loopback_http(profile.is_none()).clock(clock.clone()).settings("default")
            .studio_profile(crate::studio_bootstrap::profile(storage.retry_epochs()?.current)?)
            .blobs(blobs)
            .blob_store_discovery(|actor,name| actor.authority=="local" && actor.principal_kind()==rom::PrincipalKind::Human && name=="local")
            ;
        if profile.is_none() { config=config.provider(OidcProviderConfig {
                authority:"local".into(), label:"Local fixture".into(), issuer:issuer.into(), client_id:"studio".into(),
                authorization_endpoint:format!("{issuer}/auth"), token_endpoint:format!("{issuer}/token"), jwks_endpoint:format!("{issuer}/jwks"),
                client_secret:Some("controlled-host-test-secret".into()),
            }); }
        if let Some(profile)=profile {
            config=config.provider(profile.provider);
            if let Some(backchannel)=profile.backchannel { config=config.provider_backchannel("local",backchannel); }
        }
        let control_listener=controls.map(crate::studio_controls::listener).transpose()?;
        let host=StudioHost::new(runtime.clone(),config)?;
        let _worker=runtime.start_work()?;
        let signals=SignalReceiver::install()?;
        let control_task=control_listener.map(|listener|tokio::spawn(crate::studio_controls::run(runtime.clone(),clock,gate,listener)));
        println!("{}",serde_json::json!({"ready":true,"origin":origin,"studio":format!("{origin}/rom-studio/"),"backend":if redb{"redb"}else{"sqlite"},"fixture_controls":controls.is_some()}));
        // Private publication release remains reachable while host shutdown drains accepted work.
        let served=host.serve(listener,async move {signals.wait().await}).await;
        if let Some(task)=control_task {task.abort();let _=task.await;}
        served?;
        Ok(())
    }.await;
    let stopped = runtime.shutdown().await.map_err(Into::into);
    result.and(stopped)
}
