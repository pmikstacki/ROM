use rom::{Runtime, Storage};
use rom_demo::{Notices, bootstrap, build, resolver, smoke};
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
        "smoke" => {
            if args.next().is_some() {
                return Err("usage: rom-demo smoke [sqlite|redb]".into());
            }
            smoke::run(redb).await?;
            println!(
                "Smoke passed ({backend}): two Resource kinds, codec rejection, typed patch, TCP query/live/journal, configuration, reaction, typed notification, access denial, graceful shutdown."
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
            let _worker = runtime.start_work()?;
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            println!(
                "Local synthetic demo at http://{}; session header: Authorization: Demo local",
                listener.local_addr()?
            );
            println!("Status: {}", serde_json::to_string(&runtime.status()?)?);
            rom_http::Http::new(runtime.clone(), resolver(), Default::default())?
                .serve(listener, async {
                    if let Err(error) = tokio::signal::ctrl_c().await {
                        eprintln!("Signal listener failed: {error}");
                    }
                })
                .await?;
            println!("Stopped: {}", serde_json::to_string(&runtime.status()?)?);
        }
        _ => return Err("usage: rom-demo [smoke|serve] [sqlite|redb] [database] [port]".into()),
    }
    Ok(())
}
