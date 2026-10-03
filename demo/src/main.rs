use rom_demo::smoke;
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
        #[cfg(feature = "provider-profile")]
        "provider-provision" | "provider-maintain" | "provider-serve" => {
            rom_demo::provider_profile::run_command(&mode, redb, args.collect()).await?;
        }
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
            rom_demo::serving::run(redb, &path, port).await?;
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
