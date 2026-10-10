use crate::application::{LoadRecord, runtime, seeded};
use rom::{Resource, Storage, WorkState};
use std::sync::{Arc, atomic::AtomicUsize};
pub(crate) fn native_storage(adapter: &str) -> Arc<dyn Storage> {
    let pid = std::process::id();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("rom-010-load-registration-{adapter}-{pid}-{nonce}"));
    std::fs::create_dir(&directory).unwrap();
    match adapter {
        "sqlite" => Arc::new(rom_sqlite::Sqlite::open(directory.join("database")).unwrap()),
        "redb" => Arc::new(rom_redb::Redb::open(directory.join("database")).unwrap()),
        _ => unreachable!(),
    }
}
async fn actual_reaction_profile(adapter: &str, observed: bool) {
    let native = native_storage(adapter);
    let monitor = Arc::new(crate::ObservedStorage::new(native.clone()));
    let storage: Arc<dyn Storage> = if observed { monitor.clone() } else { native };
    let runtime = runtime(storage.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    for index in 0..10 {
        runtime
            .execute(&rom_demo::bootstrap_actor(), seeded(index))
            .await
            .unwrap();
    }
    runtime.process_reactions(32).await.unwrap();
    let work = storage.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(work.records.len(), 10, "one admitted reaction per create");
    assert!(
        work.records
            .iter()
            .all(|record| record.state == WorkState::Done),
        "seed worker actually completes every admitted reaction"
    );
    let candidate = storage
        .snapshot(LoadRecord::KIND, 12000, 32 * 1024 * 1024)
        .unwrap();
    assert_eq!(candidate.len(), 10);
    assert!(serde_json::to_vec(&candidate).unwrap().len() < 32 * 1024 * 1024);
    assert!(serde_json::to_vec(&work).unwrap().len() < 16 * 1024 * 1024);
    if observed {
        let (samples, missing, rejected) = monitor.timings().unwrap();
        assert_eq!(samples.len(), 10);
        assert_eq!(missing, 0);
        assert_eq!(rejected, 0);
    }
    runtime.shutdown().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_load_registration_admits_and_drains_actual_reactions() {
    actual_reaction_profile("sqlite", false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_load_registration_admits_and_drains_actual_reactions() {
    actual_reaction_profile("redb", false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_observed_load_registration_preserves_actual_work() {
    actual_reaction_profile("sqlite", true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_observed_load_registration_preserves_actual_work() {
    actual_reaction_profile("redb", true).await;
}
