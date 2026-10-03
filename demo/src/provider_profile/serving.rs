use super::{HostAuth, ProfileConfig, configuration_reader};
use rom::{Error, Result, Runtime};
use std::{future::Future, sync::Arc};
/// Close and drain accepted authentication work before HTTP closes Runtime.
pub async fn serve(
    runtime: Runtime,
    auth: HostAuth,
    listener: tokio::net::TcpListener,
    stop: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let draining = auth.clone();
    let result = rom_http::Http::new_async(runtime, auth.resolver(), Default::default())?
        .serve(listener, async move {
            stop.await;
            draining.close();
            let _ = draining.drain().await;
        })
        .await;
    auth.close();
    let drain = auth.drain().await;
    result.and(drain)
}

pub(super) async fn run(
    runtime: Runtime,
    clock: Arc<dyn rom::Clock>,
    config: ProfileConfig,
    port: u16,
) -> Result<()> {
    let auth = HostAuth::new(
        runtime.clone(),
        configuration_reader(),
        clock,
        config.approved,
        config.secrets,
        config.endpoint_policy,
        config.auth,
    )?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|_| Error::Storage)?;
    let address = listener.local_addr().map_err(|_| Error::Storage)?;
    // Install signal reception before reporting readiness.
    let mut termination = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|_| Error::Storage)?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(|_| Error::Storage)?;
    println!("{{\"endpoint\":\"http://{address}\"}}");
    serve(runtime, auth, listener, async move {
        tokio::select! {_ = interrupt.recv()=>{},_ = termination.recv()=>{}}
    })
    .await
}
