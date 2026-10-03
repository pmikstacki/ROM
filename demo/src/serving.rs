//! Local synthetic serving and ordered attachment/Runtime shutdown.
use crate::{
    Notices, attachments, bootstrap, build, host_signals::SignalReceiver, resolver, session_actor,
    smoke::SmokeResult,
};
use rom::{Runtime, Storage};
use rom_blob::BlobService;
use std::{path::Path, sync::Arc};

/// Serve the existing local synthetic application on a loopback listener.
pub async fn run(redb: bool, path: &str, port: u16) -> SmokeResult<()> {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path)?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path)?)
    };
    let runtime = build(storage, Notices::default())?;
    let result = async {
        bootstrap(&runtime).await?;
        let blobs = attachments::open(runtime.clone(), Path::new(&format!("{path}.objects")))?;
        serve(&runtime, blobs, port).await
    }
    .await;
    let shutdown = runtime.shutdown().await.map_err(Into::into);
    result.and(shutdown)?;
    println!("Stopped: {}", serde_json::to_string(&runtime.status()?)?);
    Ok(())
}

async fn serve(runtime: &Runtime, blobs: BlobService, port: u16) -> SmokeResult<()> {
    let result = async {
        match runtime
            .read::<rom_blob::Blob>(&session_actor(), attachments::ID)
            .await
        {
            Err(rom::Error::Missing) => attachments::attach(&blobs).await?,
            Ok(_) => {}
            Err(error) => return Err(error.into()),
        }
        let _worker = runtime.start_work()?;
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        let signals = SignalReceiver::install()?;
        println!(
            "Local synthetic demo at http://{}; session header: Authorization: Demo local",
            listener.local_addr()?
        );
        println!("Status: {}", serde_json::to_string(&runtime.status()?)?);
        let draining_blobs = blobs.clone();
        rom_http::Http::new(runtime.clone(), resolver(), Default::default())?
            .serve(listener, async move {
                signals.wait().await;
                if let Err(error) = draining_blobs.shutdown().await {
                    eprintln!("Attachment drain failed: {error}");
                }
            })
            .await?;
        Ok(())
    }
    .await;
    let drain = blobs.shutdown().await.map_err(Into::into);
    result.and(drain)
}
