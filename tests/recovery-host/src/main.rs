//! Finite, disposable HTTP host for independent mutation recovery acceptance.
mod control;
mod note;
#[cfg(test)]
mod note_tests;
mod storage;

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
    time::Duration,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 4 && args[0] == "retain" {
        let source = PathBuf::from(&args[2]);
        let destination = PathBuf::from(&args[3]);
        if !source.is_absolute() || !destination.is_absolute() {
            return Err("retention paths must be absolute".into());
        }
        storage::retire(&args[1], &source, &destination)?;
        println!(
            "{}",
            rom::json!({"retired_epoch":0,"current_epoch":1,"database":destination})
        );
        return Ok(());
    }
    if args.len() != 4 {
        return Err("usage: rom-recovery-host sqlite|redb DATABASE PORT BUDGET_MS".into());
    }
    let path = PathBuf::from(&args[1]);
    if !path.is_absolute() {
        return Err("database path must be absolute".into());
    }
    let port: u16 = args[2].parse()?;
    let budget: u64 = args[3].parse()?;
    if !(1000..=600000).contains(&budget) {
        return Err("invalid finite host budget".into());
    }
    let storage = Arc::new(storage::open(&args[0], &path)?);
    let fixture = control::Fixture {
        storage: storage.clone(),
        generation: Arc::new(AtomicU64::new(0)),
        revoked: Arc::new(AtomicBool::new(false)),
    };
    let runtime = note::declarations()
        .actor_gate(fixture.gate())
        .build(storage.store.clone(), rom::Runtime::shared_cpu_pool(1)?)?;
    let http = rom_http::Http::new(runtime, fixture.resolver(), rom_http::Limits::default())?;
    let router = http.router().merge(fixture.router());
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!(
        "{}",
        rom::json!({"address": listener.local_addr()?.to_string(), "adapter":args[0], "database":path})
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::time::sleep(Duration::from_millis(budget)).await;
        })
        .await?;
    http.shutdown().await?;
    Ok(())
}
