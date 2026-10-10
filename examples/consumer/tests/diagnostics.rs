//! Public host drainage; this file also runs from extracted consumer packages.
use rom::{
    Actor, Command, DiagnosticKey, DiagnosticOptions, DiagnosticOutcome, DiagnosticReader,
    DiagnosticStage, Diagnostics, Runtime,
};
use rom_consumer::{COMPLETE, Task, declarations};
use rom_sqlite::Sqlite;
use std::{sync::Arc, time::Duration};

const PRIVATE_TITLE: &str = "consumer-private-title-do-not-export";
const PRIVATE_ID: &str = "consumer-private-resource-do-not-export";
const PRIVATE_KEY: &str = "consumer-private-request-do-not-export";

fn setup(capacity: usize) -> (Runtime, Arc<Sqlite>, DiagnosticReader) {
    // Synthetic test key/session only. A deployed host supplies protected key material.
    let (sink, reader) = Diagnostics::bounded(
        DiagnosticOptions::new(DiagnosticKey::new([73; 32]), [19; 16]).capacity(capacity),
    )
    .unwrap();
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .diagnostics(sink)
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    (runtime, store, reader)
}

fn actor() -> Actor {
    Actor::trusted("local", "consumer-private-principal-do-not-export")
}

fn create(id: &str, key: &str) -> Command<Task> {
    Command::create(
        id,
        Task {
            owner: actor().subject,
            title: PRIVATE_TITLE.into(),
            done: false,
            note: Some("consumer-private-note-do-not-export".into()),
        },
    )
    .idempotency(key)
}

#[tokio::test]
async fn public_host_export_preserves_links_without_private_payloads() {
    let (runtime, store, mut reader) = setup(128);
    let first = runtime
        .execute(&actor(), create(PRIVATE_ID, PRIVATE_KEY))
        .await
        .unwrap();
    let replay = runtime
        .execute(&actor(), create(PRIVATE_ID, PRIVATE_KEY))
        .await
        .unwrap();
    assert_eq!(first.id, replay.id);
    assert_eq!(first.revision, replay.revision);
    assert_eq!(first.value, replay.value);
    runtime
        .execute(
            &actor(),
            Command::action(PRIVATE_ID, COMPLETE, ())
                .at_revision(1)
                .idempotency("consumer-private-action-do-not-export"),
        )
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
    reader.close();
    let mut records = Vec::new();
    while let Some(record) = reader.try_recv() {
        assert!(records.len() < 128);
        records.push(record);
    }
    assert!(
        records.iter().any(
            |r| r.stage == DiagnosticStage::Commit && r.outcome == DiagnosticOutcome::Succeeded
        )
    );
    assert!(
        records
            .iter()
            .any(|r| r.stage == DiagnosticStage::Receipt && r.outcome == DiagnosticOutcome::Replay)
    );
    assert!(records.iter().any(|r| r.stage == DiagnosticStage::Event));
    let commits: Vec<_> = records
        .iter()
        .filter(|r| r.stage == DiagnosticStage::Commit && r.outcome == DiagnosticOutcome::Succeeded)
        .collect();
    assert_eq!(commits.len(), 2);
    let replay = records
        .iter()
        .find(|r| r.outcome == DiagnosticOutcome::Replay)
        .unwrap();
    assert_eq!(commits[0].root, replay.root);
    assert_eq!(commits[0].operation, replay.operation);
    assert!(replay.sequence > commits[0].sequence);
    let exported = serde_json::to_string(&records).unwrap();
    for private in [
        PRIVATE_TITLE,
        PRIVATE_ID,
        PRIVATE_KEY,
        "consumer-private-principal-do-not-export",
        "consumer-private-note-do-not-export",
        "consumer-private-action-do-not-export",
    ] {
        assert!(!exported.contains(private));
    }
    assert_eq!(store.counts().unwrap(), [1, 2, 2, 1]);
    assert_eq!(reader.stats().dropped_full, 0);
}

#[tokio::test]
async fn full_or_closed_host_reader_does_not_block_commands() {
    let (runtime, store, mut reader) = setup(1);
    for index in 0..2 {
        tokio::time::timeout(
            Duration::from_secs(2),
            runtime.execute(&actor(), create(&format!("task-{index}"), "create")),
        )
        .await
        .unwrap()
        .unwrap();
    }
    assert!(reader.stats().dropped_full > 0);
    reader.close();
    tokio::time::timeout(
        Duration::from_secs(2),
        runtime.execute(&actor(), create("task-after-disposal", "create")),
    )
    .await
    .unwrap()
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(store.counts().unwrap(), [3, 3, 3, 0]);
}

#[tokio::test]
async fn stalled_then_failed_host_exporter_cannot_reject_a_committed_command() {
    let (runtime, store, mut reader) = setup(128);
    let (started, waiting) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    // Exporter lifetime is host-owned. It holds the reader, never a Runtime or database.
    let exporter = tokio::spawn(async move {
        let record = reader.recv().await.unwrap();
        let bytes = serde_json::to_vec(&record).unwrap();
        started.send(()).unwrap();
        released.await.unwrap();
        let failed_export: std::io::Result<()> =
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        reader.close();
        (bytes, failed_export)
    });
    runtime
        .execute(&actor(), create(PRIVATE_ID, PRIVATE_KEY))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        runtime.execute(&actor(), create("task-while-exporter-stalled", "create")),
    )
    .await
    .unwrap()
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), runtime.shutdown())
        .await
        .unwrap()
        .unwrap();
    release.send(()).unwrap();
    let (bytes, result) = exporter.await.unwrap();
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::BrokenPipe);
    assert!(!String::from_utf8(bytes).unwrap().contains(PRIVATE_TITLE));
    assert_eq!(store.counts().unwrap(), [2, 2, 2, 0]);
}
