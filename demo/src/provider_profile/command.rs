//! Thin local command composition over explicit host configuration.
use super::{
    LocalMode, ProfileConfig, build, configuration::read_document, maintain, provision, serving,
};
use rom::{Error, Result, Storage};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Local commands accept paths only. Credential material never becomes arguments.
/// Serving uses the current provider Resource; it never provisions automatically.
pub async fn run_command(mode: &str, redb: bool, args: Vec<String>) -> Result<()> {
    let expected = if mode == "provider-maintain" { 3 } else { 2 };
    if args.len() < expected
        || args.len() > expected + usize::from(mode == "provider-serve")
        || !matches!(
            mode,
            "provider-provision" | "provider-maintain" | "provider-serve"
        )
    {
        return Err(Error::Denied);
    }
    let config = ProfileConfig::load(Path::new(&args[1]))?;
    let invocation = if mode == "provider-maintain" {
        Some(
            serde_json::from_value(read_document(Path::new(&args[2]), 16_384)?)
                .map_err(|_| Error::Denied)?,
        )
    } else {
        None
    };
    let port = if mode == "provider-serve" {
        args.get(2)
            .map(|v| v.parse::<u16>().map_err(|_| Error::Denied))
            .transpose()?
            .unwrap_or(8080)
    } else {
        0
    };
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(PathBuf::from(&args[0]))?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(PathBuf::from(&args[0]))?)
    };
    let clock: Arc<dyn rom::Clock> = Arc::new(rom::SystemClock);
    let runtime = build(
        storage,
        clock.clone(),
        &config.provisioning,
        if mode == "provider-provision" {
            LocalMode::Provisioning
        } else {
            LocalMode::Serving
        },
    )?;
    let result = async {
        match mode {
            "provider-provision" => {
                provision(&runtime, &config.provisioning).await?;
                println!("{{\"provisioned\":true}}");
                Ok(())
            }
            "provider-maintain" => {
                let row = maintain(
                    &runtime,
                    &config.provisioning,
                    invocation.ok_or(Error::Denied)?,
                )
                .await?;
                println!("{{\"revision\":{}}}", row.revision);
                Ok(())
            }
            _ => serving::run(runtime.clone(), clock, config, port).await,
        }
    }
    .await;
    let shutdown = runtime.shutdown().await;
    result.and(shutdown)
}
