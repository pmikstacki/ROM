//! Retain real Runtime receipts and the immutable source backup.
use crate::model;
use rom::*;
use rom_backup::{Backend, BackupLimits};
use std::{path::Path, sync::Arc};

pub async fn run() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: writer sqlite|redb DIRECTORY");
    let root = Path::new(&args[2]);
    assert!(root.is_dir());
    let source = root.join("source");
    let archive = root.join("before.rombk");
    assert!(!source.exists() && !archive.exists() && !root.join("summary.json").exists());
    let backend = match args[1].as_str() {
        "sqlite" => Backend::Sqlite,
        "redb" => Backend::Redb,
        _ => panic!("unsupported backend"),
    };
    let store: Arc<dyn Storage> = match backend {
        Backend::Sqlite => Arc::new(rom_sqlite::Sqlite::open(&source).unwrap()),
        Backend::Redb => Arc::new(rom_redb::Redb::open(&source).unwrap()),
    };
    let runtime = Runtime::builder()
        .resource(model::definition(true))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let mut results = Vec::new();
    for command in model::commands() {
        let result = runtime.execute(&model::actor(), command).await.unwrap();
        results.push(json!({"id": result.id, "revision": result.revision, "value": result.value.unwrap().encode()}));
    }
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(store);
    match backend {
        Backend::Sqlite => rom_sqlite::Sqlite::open(&source)
            .unwrap()
            .backup_to(&archive, BackupLimits::default())
            .unwrap(),
        Backend::Redb => rom_redb::Redb::open(&source)
            .unwrap()
            .backup_to(&archive, BackupLimits::default())
            .unwrap(),
    };
    let (manifest, snapshot) =
        rom_backup::read(&archive, backend, BackupLimits::default()).unwrap();
    assert_eq!((manifest.archive_version, manifest.storage_format), (6, 8));
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.receipts.len(), 3);
    assert_eq!(snapshot.events.len(), 3);
    assert_eq!(snapshot.effects.len(), 1);
    assert_eq!(
        snapshot.rows[0].value,
        Some(json!({"code":"SECOND","quantity":5}))
    );
    std::fs::write(
        root.join("summary.json"),
        json!({"writer_release":"0.0.3", "results":results,"snapshot":snapshot}).to_string(),
    )
    .unwrap();
    println!(
        "actual accepted Runtime custom-codec commands verified {}",
        args[1]
    );
}
