use rom::{Runtime, Storage};
use rom_demo::{Notices, attachments, bootstrap, build, resolver, smoke};
use std::sync::Arc;
#[tokio::main]
async fn main() -> smoke::SmokeResult<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "smoke".into());
    let backend = args.next().unwrap_or_else(|| "sqlite".into());
    if !matches!(backend.as_str(), "sqlite" | "redb") {
        return Err("backend must be sqlite or redb".into());
    }
    let redb = backend == "redb";
    match mode.as_str() {
        "operator" => {
            if args.next().is_some() {
                return Err("usage: rom-demo operator [sqlite|redb]".into());
            }
            rom_demo::operator::run(redb).await?;
            println!(
                "Operator recovery passed ({backend}): denied delivery, restart, authorized unchanged retry, receipt replay and explicit Resource compensation."
            );
        }
        "upgrade" => {
            if args.next().is_some() {
                return Err("usage: rom-demo upgrade [sqlite|redb]".into());
            }
            rom_demo::upgrade::run(redb).await?;
            println!(
                "Upgrade recovery passed ({backend}): old schema, pending work, migration, backup/restore, retained codec replay, rebuilt restrict references and external attachment bytes."
            );
        }
        "reference" => {
            if args.next().is_some() {
                return Err("usage: rom-demo reference [sqlite|redb]".into());
            }
            rom_demo::reference::run(redb).await?;
            println!(
                "Reference recovery passed ({backend}): pending compensation, preserved unrelated reservation, replay without duplicate events, typed ordered pages, live membership and task reaction."
            );
        }
        "smoke" => {
            if args.next().is_some() {
                return Err("usage: rom-demo smoke [sqlite|redb]".into());
            }
            smoke::run(redb).await?;
            println!(
                "Smoke passed ({backend}): two Resource kinds, codec rejection, typed patch, TCP query/live/journal, configuration, reaction, explicit compensation, typed notification, access denial, folder attachment/reopen/detach, graceful shutdown."
            );
        }
        "serve" => {
            let path = args.next().unwrap_or_else(|| format!("demo.{backend}.db"));
            let port: u16 = args.next().map(|s| s.parse()).transpose()?.unwrap_or(8080);
            if args.next().is_some() {
                return Err("usage: rom-demo serve [sqlite|redb] [database] [port]".into());
            }
            let storage: Arc<dyn Storage> = if redb {
                Arc::new(rom_redb::Redb::open(&path)?)
            } else {
                Arc::new(rom_sqlite::Sqlite::open(&path)?)
            };
            let runtime: Runtime = build(storage, Notices::default())?;
            bootstrap(&runtime).await?;
            let blobs = attachments::open(
                runtime.clone(),
                std::path::Path::new(&format!("{path}.objects")),
            )?;
            match runtime
                .read::<rom_blob::Blob>(&rom_demo::session_actor(), attachments::ID)
                .await
            {
                Err(rom::Error::Missing) => attachments::attach(&blobs).await?,
                Ok(_) => {}
                Err(error) => return Err(error.into()),
            }
            let _worker = runtime.start_work()?;
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            println!(
                "Local synthetic demo at http://{}; session header: Authorization: Demo local",
                listener.local_addr()?
            );
            println!("Status: {}", serde_json::to_string(&runtime.status()?)?);
            let draining_blobs = blobs.clone();
            rom_http::Http::new(runtime.clone(), resolver(), Default::default())?
                .serve(listener, async move {
                    if let Err(error) = tokio::signal::ctrl_c().await {
                        eprintln!("Signal listener failed: {error}");
                    }
                    if let Err(error) = draining_blobs.shutdown().await {
                        eprintln!("Attachment drain failed: {error}");
                    }
                })
                .await?;
            blobs.shutdown().await?;
            println!("Stopped: {}", serde_json::to_string(&runtime.status()?)?);
        }
        _ => {
            return Err(
                "usage: rom-demo [smoke|reference|operator|upgrade|serve] [sqlite|redb] [database] [port]"
                    .into(),
            );
        }
    }
    Ok(())
}
