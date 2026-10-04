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
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path)?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path)?)
    };
    let clock = Arc::new(DemoClock {
        offset: AtomicU64::new(0),
    });
    let runtime = studio_application::build(storage, clock.clone())?;
    let result=async {
  studio_startup::seed_all(&runtime,issuer).await?;
  let listener=tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,port)).await?;
  let origin=format!("http://{}",listener.local_addr()?);
  let config=HostConfig::new(&origin,"/rom-studio/",assets,studio_application::host_actor()).allow_loopback_http(true).clock(clock.clone()).settings("default").provider(OidcProviderConfig{authority:"local".into(),label:"Local fixture".into(),issuer:issuer.into(),client_id:"studio".into(),authorization_endpoint:format!("{issuer}/auth"),token_endpoint:format!("{issuer}/token"),jwks_endpoint:format!("{issuer}/jwks"),client_secret:Some("controlled-host-test-secret".into())});
  let control_listener=controls.map(crate::studio_controls::listener).transpose()?;
  let host=StudioHost::new(runtime.clone(),config)?;
  let _worker=runtime.start_work()?;
  let signals=SignalReceiver::install()?;
  println!("{}",serde_json::json!({"ready":true,"origin":origin,"studio":format!("{origin}/rom-studio/"),"backend":if redb{"redb"}else{"sqlite"},"fixture_controls":controls.is_some()}));
  let controlled_runtime=runtime.clone();
  host.serve(listener,async move{if let Some(listener)=control_listener{tokio::select!{_ = signals.wait()=>{},_ = crate::studio_controls::run(controlled_runtime,clock,listener)=>{}}}else{signals.wait().await;}}).await?;
  Ok(())
 }.await;
    let stopped = runtime.shutdown().await.map_err(Into::into);
    result.and(stopped)
}
