//! Real Runtime and file-backed adapters establish literal histogram expectations.
use crate::{ObservedStorage, application};
use rom::{Command, Storage, WorkState};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

async fn dense_sources(adapter: &str) {
    let native = crate::application_tests::native_storage(adapter);
    let observed = Arc::new(ObservedStorage::new(native.clone()));
    let runtime = application::runtime(observed.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    for index in 0..16 {
        runtime
            .execute(&rom_demo::bootstrap_actor(), application::seeded(index))
            .await
            .unwrap();
    }
    assert_eq!(runtime.process_reactions(16).await.unwrap(), 16);
    let work = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(work.records.len(), 16);
    assert!(
        work.records
            .iter()
            .all(|record| record.state == WorkState::Done)
    );
    assert_eq!(runtime.process_reactions(1).await.unwrap(), 0);
    runtime.shutdown().await.unwrap();
    let histogram = observed.work_batches().unwrap();
    assert!(!histogram.unavailable);
    assert_eq!(histogram.prefix.len(), 33);
    assert_eq!(histogram.atomic.len(), 33);
    assert_eq!(histogram.prefix[16].calls, 1);
    assert_eq!(histogram.prefix[16].source_claims, 16);
    assert_eq!(histogram.prefix[16].notification_claims, 0);
    assert_eq!(histogram.prefix[0].calls, 1);
    assert_eq!(histogram.atomic[16].calls, 1);
    assert_eq!(
        histogram.prefix.iter().map(|cell| cell.calls).sum::<u64>(),
        2
    );
    assert_eq!(
        histogram.atomic.iter().map(|cell| cell.calls).sum::<u64>(),
        1
    );
    assert!(
        histogram
            .prefix
            .iter()
            .chain(&histogram.atomic)
            .all(|cell| cell.failures == 0)
    );
}

#[tokio::test]
async fn sqlite_dense_source_backlog_records_real_width_sixteen_and_idle_zero() {
    dense_sources("sqlite").await;
}

#[tokio::test]
async fn redb_dense_source_backlog_records_real_width_sixteen_and_idle_zero() {
    dense_sources("redb").await;
}

async fn notification_barrier(adapter: &str) {
    let native = crate::application_tests::native_storage(adapter);
    let observed = Arc::new(ObservedStorage::new(native.clone()));
    let deliveries = Arc::new(AtomicUsize::new(0));
    let runtime = application::runtime(observed.clone(), deliveries.clone()).unwrap();
    runtime
        .execute(&rom_demo::bootstrap_actor(), application::seeded(0))
        .await
        .unwrap();
    assert_eq!(runtime.process_reactions(32).await.unwrap(), 1);
    runtime
        .execute(
            &rom_demo::bootstrap_actor(),
            Command::action("load-00000", application::TOUCH, true)
                .idempotency("width-notification")
                .at_revision(1),
        )
        .await
        .unwrap();
    assert_eq!(runtime.process_reactions(32).await.unwrap(), 2);
    runtime.shutdown().await.unwrap();
    assert_eq!(deliveries.load(Ordering::Relaxed), 1);
    let work = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(work.records.len(), 3);
    assert!(
        work.records
            .iter()
            .all(|record| record.state == WorkState::Done)
    );
    let histogram = observed.work_batches().unwrap();
    assert!(!histogram.unavailable);
    assert_eq!(histogram.prefix[1].calls, 3);
    assert_eq!(histogram.prefix[1].source_claims, 2);
    assert_eq!(histogram.prefix[1].notification_claims, 1);
    assert_eq!(histogram.prefix[0].calls, 2);
    assert_eq!(histogram.atomic[1].calls, 2);
    assert_eq!(
        histogram.prefix.iter().map(|cell| cell.calls).sum::<u64>(),
        5
    );
    assert_eq!(
        histogram.atomic.iter().map(|cell| cell.calls).sum::<u64>(),
        2
    );
}

#[tokio::test]
async fn sqlite_notification_keeps_source_and_delivery_claims_as_ordered_singletons() {
    notification_barrier("sqlite").await;
}

#[tokio::test]
async fn redb_notification_keeps_source_and_delivery_claims_as_ordered_singletons() {
    notification_barrier("redb").await;
}

#[cfg(feature = "storage-stage-timings")]
#[tokio::test]
async fn unknown_prefix_ack_is_preserved_and_is_not_counted_as_an_idle_width() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "rom-load-width-unknown-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let native = Arc::new(rom_sqlite::Sqlite::open(directory.join("database")).unwrap());
    let observed = Arc::new(ObservedStorage::new(native.clone()));
    let runtime = application::runtime(observed.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    runtime
        .execute(&rom_demo::bootstrap_actor(), application::seeded(0))
        .await
        .unwrap();
    let before = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    let now = before.records[0].pending.cause.started_at;
    native.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == usize::MAX {
            Err(rom::Error::Storage)
        } else {
            Ok(())
        }
    })));
    assert_eq!(
        observed.reaction_claim_prefix(now, 16),
        Err(rom::Error::Unknown)
    );
    native.on_commit(None);
    let work = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(work.records.len(), 1);
    assert!(matches!(work.records[0].state, WorkState::Leased { .. }));
    runtime.shutdown().await.unwrap();
    let histogram = observed.work_batches().unwrap();
    assert!(!histogram.unavailable);
    assert_eq!(histogram.prefix_failed_calls, 1);
    assert_eq!(
        histogram.prefix.iter().map(|cell| cell.calls).sum::<u64>(),
        0
    );
    drop(runtime);
    drop(observed);
    drop(native);
    let reopened = rom_sqlite::Sqlite::open(directory.join("database")).unwrap();
    assert_eq!(
        reopened.work_snapshot(12000, 16 * 1024 * 1024).unwrap(),
        work
    );
}
