//! Exercise the actual load declaration and its emitted storage request.
use crate::ObservedStorage;
use crate::application::{LoadRecord, runtime, seeded};
use rom::{Error, QuerySpec, Resource, SelectionMode};
use std::sync::{Arc, atomic::AtomicUsize};

async fn fixture(adapter: &str) -> (rom::Runtime, Arc<ObservedStorage>) {
    let store = crate::application_tests::native_storage(adapter);
    let observer = Arc::new(ObservedStorage::new(store));
    let runtime = runtime(observer.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    for index in 0..8 {
        runtime
            .execute(&rom_demo::bootstrap_actor(), seeded(index))
            .await
            .unwrap();
    }
    (runtime, observer)
}

async fn admitted_query(adapter: &str) {
    let (runtime, observer) = fixture(adapter).await;
    let result = runtime
        .query_spec_projected(
            &rom_demo::bootstrap_actor(),
            LoadRecord::KIND,
            QuerySpec::equal("title", rom::json!("Synthetic load row 00003")).limit(50),
        )
        .await;
    runtime.shutdown().await.unwrap();
    let rows = result.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.id, "load-00003");
    assert_eq!(rows[0].revision, 1);
    assert_eq!(
        serde_json::to_value(&rows[0].value).unwrap(),
        rom::json!({"title":"Synthetic load row 00003","counter":0,"open":false})
    );
    let (request, bounds) = observer.take_query_request().unwrap();
    assert_eq!(request.selection, SelectionMode::UniformReadAndFields);
    assert_eq!(bounds.max_rows, 12000);
    assert_eq!(bounds.max_bytes, 32 * 1024 * 1024);
}

async fn denied_query(adapter: &str) {
    let (runtime, observer) = fixture(adapter).await;
    // This principal passes the demo IdentityGate but lacks load Resource access.
    let result = runtime
        .query_spec_projected(
            &rom_demo::session_actor(),
            LoadRecord::KIND,
            QuerySpec::all().limit(50),
        )
        .await;
    runtime.shutdown().await.unwrap();
    assert_eq!(result.unwrap_err(), Error::Denied);
    assert!(observer.take_query_request().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_load_read_policy_emits_uniform_selection() {
    admitted_query("sqlite").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_load_read_policy_emits_uniform_selection() {
    admitted_query("redb").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_load_read_policy_denies_before_query_storage() {
    denied_query("sqlite").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_load_read_policy_denies_before_query_storage() {
    denied_query("redb").await;
}

async fn write_policy(adapter: &str) {
    let (runtime, observer) = fixture(adapter).await;
    let worker = rom::Actor::trusted("demo-host", "worker").with_kind(rom::PrincipalKind::Service);
    let changed = runtime
        .execute(
            &worker,
            rom::Command::action("load-00003", crate::application::TOUCH, false)
                .idempotency("allowed-touch")
                .at_revision(1),
        )
        .await
        .unwrap();
    assert_eq!(changed.revision, 2);
    assert_eq!(changed.value.as_ref().unwrap().counter, 1);
    let rejected = runtime
        .execute(
            &rom_demo::session_actor(),
            rom::Command::action("load-00003", crate::application::TOUCH, false)
                .idempotency("denied-touch")
                .at_revision(2),
        )
        .await;
    let current = runtime
        .read::<LoadRecord>(&rom_demo::bootstrap_actor(), "load-00003")
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
    assert!(matches!(rejected, Err(Error::Denied)));
    assert_eq!(current.revision, changed.revision);
    assert_eq!(
        current.value.as_ref().unwrap().encode(),
        changed.value.as_ref().unwrap().encode()
    );
    assert!(observer.take_query_request().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_load_read_policy_preserves_write_rules() {
    write_policy("sqlite").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_load_read_policy_preserves_write_rules() {
    write_policy("redb").await;
}

#[cfg(feature = "storage-stage-timings")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_load_read_policy_measures_captured_request_and_full_kind_bounds() {
    use rom::{QueryRead, QueryStrategy};
    use rom_sqlite::{QueryExecution, Sqlite};

    let sqlite = Arc::new(Sqlite::open(":memory:").unwrap());
    let observer = Arc::new(ObservedStorage::new(sqlite.clone()));
    let runtime = runtime(observer.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    for index in 0..128 {
        runtime
            .execute(&rom_demo::bootstrap_actor(), seeded(index))
            .await
            .unwrap();
    }
    let result = runtime
        .query_spec_projected(
            &rom_demo::bootstrap_actor(),
            LoadRecord::KIND,
            QuerySpec::equal("title", rom::json!("Synthetic load row 00017")).limit(1),
        )
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].key.id, "load-00017");
    let (request, bounds) = observer.take_query_request().unwrap();
    assert_eq!(request.selection, SelectionMode::UniformReadAndFields);

    // Independent adapter observations of the actual emitted request. These are
    // not telemetry of the earlier Runtime call or of the mixed load trial.
    let automatic = sqlite
        .query_read_observed(&request, bounds, QueryExecution::Automatic)
        .unwrap();
    let reference = sqlite
        .query_read_observed(&request, bounds, QueryExecution::Reference)
        .unwrap();
    assert_eq!(automatic.metrics.strategy, QueryStrategy::NativeCandidates);
    assert_eq!(automatic.metrics.decoded_rows, 1);
    assert_eq!(reference.metrics.decoded_rows, 128);
    assert!(automatic.metrics.decoded_bytes < reference.metrics.decoded_bytes);
    eprintln!(
        "captured-request adapter observations: automatic={:?}; reference={:?}",
        automatic.metrics, reference.metrics
    );
    let QueryRead::NativeCandidates { rows, .. } = automatic.read else {
        panic!("selective automatic query must choose admitted native candidates");
    };
    let QueryRead::Reference { rows: all } = reference.read else {
        panic!("reference observation must retain the full candidate set");
    };
    let expected: Vec<_> = all
        .into_iter()
        .filter(|row| row.key.id == "load-00017")
        .collect();
    assert_eq!(rows, expected);
    for mode in [QueryExecution::Automatic, QueryExecution::Reference] {
        assert_eq!(
            sqlite
                .query_read_observed(
                    &request,
                    rom::QueryBounds {
                        max_rows: 127,
                        ..bounds
                    },
                    mode,
                )
                .unwrap_err(),
            Error::TooLarge
        );
    }
}
