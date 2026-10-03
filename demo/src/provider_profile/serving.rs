use super::HostAuth;
use rom::{Result, Runtime};
use std::future::Future;
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
