//! Public diagnostics contract on actual SQLite and redb.
use rom::*;
#[path = "../support/diagnostic_failure.rs"]
mod diagnostic_failure;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Resource)]
#[resource(name = "DIAGNOSTIC_KIND_SECRET")]
struct Source {
    enabled: bool,
    title: String,
}
#[derive(Clone, Resource)]
#[resource(name = "diagnostic-mirrors")]
struct Mirror {
    enabled: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "diagnostic-private")]
struct Private {
    owner: String,
    secret: String,
}
const MAIL: Channel<String> = Channel::new("diagnostic-mail", 1);
const SET_MIRROR: Action<Mirror, bool> = Action::new("set", |state, enabled| {
    state.enabled = enabled;
    Ok(vec![MAIL.intent("EXTERNAL_PAYLOAD_SECRET".to_owned())])
});

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-diagnostics-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn database(&self) -> PathBuf {
        self.0.join("database")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

enum Native {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Native {
    fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(store) => store.clone(),
            Self::Redb(store) => store.clone(),
        }
    }
    fn lose_commit_ack(&self) {
        let observer = Arc::new(|point| {
            if point == usize::MAX {
                Err(Error::NotCommitted)
            } else {
                Ok(())
            }
        });
        match self {
            Self::Sqlite(store) => store.on_commit(Some(observer)),
            Self::Redb(store) => store.on_commit(Some(observer)),
        }
    }
}
fn diagnostics(capacity: usize, key: u8, session: u8) -> (DiagnosticSink, DiagnosticReader) {
    Diagnostics::bounded(
        DiagnosticOptions::new(DiagnosticKey::new([key; 32]), [session; 16]).capacity(capacity),
    )
    .unwrap()
}
fn builder(sink: DiagnosticSink) -> Builder {
    Runtime::builder().diagnostics(sink).resource(
        Source::definition()
            .allow_all_fields()
            .policy(|_, _, _| true),
    )
}
fn build(builder: Builder, native: &Native) -> Runtime {
    builder
        .build(native.storage(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
fn owner() -> Actor {
    Actor::trusted("ISSUER_SECRET", "PRINCIPAL_SECRET")
}
fn service() -> Actor {
    Actor::trusted("ISSUER_SECRET", "SERVICE_SECRET").with_kind(PrincipalKind::Service)
}
fn source(idempotency: &str) -> Command<Source> {
    Command::create(
        "RESOURCE_ID_SECRET",
        Source {
            enabled: true,
            title: "RESOURCE_PAYLOAD_SECRET".into(),
        },
    )
    .idempotency(idempotency)
}
fn drain(reader: &mut DiagnosticReader) -> Vec<DiagnosticEvent> {
    let mut records = Vec::new();
    while let Some(record) = reader.try_recv() {
        records.push(record);
    }
    records
}
fn assert_redacted(records: &[DiagnosticEvent]) {
    assert!(!records.is_empty());
    let text = serde_json::to_string(records).unwrap();
    for sentinel in [
        "DIAGNOSTIC_KIND_SECRET",
        "ISSUER_SECRET",
        "PRINCIPAL_SECRET",
        "SERVICE_SECRET",
        "RESOURCE_ID_SECRET",
        "RESOURCE_PAYLOAD_SECRET",
        "IDEMPOTENCY_SECRET",
        "EXTERNAL_PAYLOAD_SECRET",
        "PRIVATE_OWNER_SECRET",
        "PRIVATE_VALUE_SECRET",
    ] {
        assert!(
            !text.contains(sentinel),
            "diagnostic disclosure: {sentinel}"
        );
    }
}

// Catches reporting Unknown as rollback/success, unstable receipt links, or duplicate mutation on recovery.
#[tokio::test]
async fn unregistered_resource_failure_is_not_an_authorization_failure() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 34, 1);
        let runtime = build(builder(sink), &native);
        let mut invocation: Invocation = source("unregistered-probe").into();
        invocation.kind = "unknown-diagnostic-resource".into();
        assert!(matches!(
            runtime.invoke(&owner(), invocation).await,
            Err(Error::Unregistered)
        ));
        let records = drain(&mut reader);
        assert!(records.iter().any(|record| {
            record.outcome == DiagnosticOutcome::Invalid && record.stage == DiagnosticStage::Resolve
        }));
        assert!(!records.iter().any(|record| {
            record.stage == DiagnosticStage::Authorization
                && record.outcome == DiagnosticOutcome::Invalid
        }));
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn failed_storage_and_receipt_reads_keep_their_actual_phase() {
    for redb in [false, true] {
        for (mode, expected) in [
            (1, DiagnosticStage::StorageRead),
            (2, DiagnosticStage::Receipt),
        ] {
            let scratch = Scratch::new();
            let native = Native::open(redb, &scratch.database());
            let failing = Arc::new(diagnostic_failure::ReadFailure {
                inner: native.storage(),
                mode: AtomicU64::new(0),
            });
            let (sink, mut reader) = diagnostics(512, 35, 1);
            let runtime = builder(sink)
                .build(failing.clone(), Runtime::shared_cpu_pool(1).unwrap())
                .unwrap();
            failing.mode.store(mode, Ordering::SeqCst);
            assert!(matches!(
                runtime.execute(&owner(), source("failed-read")).await,
                Err(Error::Storage)
            ));
            let records = drain(&mut reader);
            assert!(
                records.iter().any(|record| record.stage == expected
                    && record.outcome == DiagnosticOutcome::Unknown)
            );
            assert!(
                !records
                    .iter()
                    .any(|record| record.stage == DiagnosticStage::Authorization
                        && record.outcome == DiagnosticOutcome::Unknown)
            );
            assert!(
                native
                    .storage()
                    .snapshot(Source::KIND, 16, 100_000)
                    .unwrap()
                    .is_empty()
            );
            assert_redacted(&records);
            runtime.shutdown().await.unwrap();
        }
    }
}

#[tokio::test]
async fn stale_revision_is_validation_conflict_after_successful_receipt_lookup() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 36, 1);
        let runtime = build(builder(sink), &native);
        runtime
            .execute(&owner(), source("revision-seed"))
            .await
            .unwrap();
        drain(&mut reader);
        let command = Command::replace(
            "RESOURCE_ID_SECRET",
            Source {
                enabled: false,
                title: "new".into(),
            },
        )
        .at_revision(999)
        .idempotency("stale-revision");
        assert!(matches!(
            runtime.execute(&owner(), command).await,
            Err(Error::Conflict)
        ));
        let records = drain(&mut reader);
        assert!(
            records
                .iter()
                .any(|record| record.stage == DiagnosticStage::Validation
                    && record.outcome == DiagnosticOutcome::Conflict)
        );
        assert!(
            !records
                .iter()
                .any(|record| record.stage == DiagnosticStage::Receipt
                    && record.outcome == DiagnosticOutcome::Conflict)
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn unknown_commit_reopen_resolves_same_receipt_without_another_event() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 23, 1);
        let runtime = build(builder(sink), &native);
        native.lose_commit_ack();
        assert!(matches!(
            runtime
                .execute(&owner(), source("IDEMPOTENCY_SECRET"))
                .await,
            Err(Error::Unknown)
        ));
        let before = drain(&mut reader);
        let unknown = before
            .iter()
            .find(|r| r.stage == DiagnosticStage::Commit && r.outcome == DiagnosticOutcome::Unknown)
            .expect("lost commit acknowledgement must be observable as unknown");
        assert_eq!(
            before
                .iter()
                .filter(|r| r.stage == DiagnosticStage::Commit
                    && r.outcome == DiagnosticOutcome::Unknown)
                .count(),
            1
        );
        assert!(
            !before.iter().any(|r| r.stage == DiagnosticStage::Execution
                && r.outcome == DiagnosticOutcome::Unknown)
        );
        let operation = unknown.operation;
        let root = unknown.root;
        assert!(!before.iter().any(|r| {
            r.operation == operation
                && r.stage == DiagnosticStage::Commit
                && r.outcome == DiagnosticOutcome::Succeeded
        }));
        assert_redacted(&before);
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(native);
        // The old reader remains alive. It must not keep native database ownership alive.
        let native = Native::open(redb, &scratch.database());
        let (sink, mut recovered_reader) = diagnostics(512, 23, 2);
        let recovered = build(builder(sink), &native);
        let result = recovered
            .execute(&owner(), source("IDEMPOTENCY_SECRET"))
            .await
            .unwrap();
        assert_eq!(result.revision, 1);
        let replay = drain(&mut recovered_reader);
        assert!(replay.iter().any(|r| {
            r.stage == DiagnosticStage::Receipt
                && r.outcome == DiagnosticOutcome::Replay
                && r.operation == operation
                && r.root == root
                && r.session == [2; 16]
        }));
        assert_eq!(
            native
                .storage()
                .journal(Source::KIND, None, 16, 100_000)
                .unwrap()
                .events
                .len(),
            1
        );
        assert_redacted(&replay);
        recovered.shutdown().await.unwrap();
    }
}

// Catches waiting for diagnostic capacity or retaining a database through a disposed reader.
#[tokio::test]
async fn full_or_closed_diagnostics_preserves_commit_and_bounded_shutdown() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(1, 24, 1);
        let runtime = build(builder(sink), &native);
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            runtime.execute(&owner(), source("IDEMPOTENCY_SECRET")),
        )
        .await
        .expect("a full diagnostic queue cannot stall a commit")
        .unwrap();
        assert_eq!(result.revision, 1);
        assert!(reader.stats().dropped_full > 0);
        reader.close();
        let replay = tokio::time::timeout(
            Duration::from_secs(5),
            runtime.execute(&owner(), source("IDEMPOTENCY_SECRET")),
        )
        .await
        .expect("closed diagnostic reader cannot stall receipt replay")
        .unwrap();
        assert_eq!(replay.revision, 1);
        tokio::time::timeout(Duration::from_secs(5), runtime.shutdown())
            .await
            .expect("runtime shutdown is independent of exporter drainage")
            .unwrap();
        drop(runtime);
        drop(native);
        let reopened = Native::open(redb, &scratch.database());
        assert_eq!(
            reopened
                .storage()
                .load(&Key {
                    kind: Source::KIND.into(),
                    id: "RESOURCE_ID_SECRET".into()
                })
                .unwrap()
                .unwrap()
                .revision,
            1
        );
        assert_redacted(&drain(&mut reader));
    }
}

fn copy(snapshot: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    Ok(vec![Target::new(
        "mirror",
        snapshot.value.as_ref().ok_or(Error::Missing)?.enabled,
    )])
}
fn chain_builder(sink: DiagnosticSink) -> Builder {
    builder(sink)
        .resource(
            Mirror::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(SET_MIRROR),
        )
        .reaction(
            Reaction::new("diagnostic-copy", 1, service(), SET_MIRROR, copy)
                .depends_on(Source::enabled_field()),
        )
        .channel(MAIL, service(), |delivery: Delivery<String>| async move {
            assert_eq!(delivery.payload, "EXTERNAL_PAYLOAD_SECRET");
            DeliveryOutcome::Accepted
        })
}

// Catches losing causal root/parent links between native reaction, child commit and external attempt after restart.
#[tokio::test]
async fn root_child_and_external_attempt_keep_correlation_after_reopen() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 25, 1);
        let runtime = build(chain_builder(sink), &native);
        runtime
            .execute(
                &owner(),
                Command::create("mirror", Mirror { enabled: false }).idempotency("seed-mirror"),
            )
            .await
            .unwrap();
        drain(&mut reader);
        runtime
            .execute(&owner(), source("IDEMPOTENCY_SECRET"))
            .await
            .unwrap();
        let origin = drain(&mut reader);
        let root = origin
            .iter()
            .find(|r| {
                r.stage == DiagnosticStage::Commit && r.outcome == DiagnosticOutcome::Succeeded
            })
            .expect("source commit diagnostic")
            .root;
        assert_eq!(runtime.process_work(1).await.unwrap(), 1);
        let mapping = drain(&mut reader);
        assert!(
            mapping
                .iter()
                .any(|r| r.root == root && r.stage == DiagnosticStage::Reaction)
        );
        let mapper_work = mapping
            .iter()
            .find_map(|r| {
                (r.root == root && r.stage == DiagnosticStage::Reaction)
                    .then_some(r.work)
                    .flatten()
            })
            .expect("mapper diagnostic contains its work link");
        assert!(
            mapping
                .iter()
                .any(|r| r.stage == DiagnosticStage::Materialize
                    && r.outcome == DiagnosticOutcome::Succeeded)
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(native);
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 25, 2);
        let runtime = build(chain_builder(sink), &native);
        assert_eq!(runtime.process_work(8).await.unwrap(), 2);
        let linked = drain(&mut reader);
        let child = linked
            .iter()
            .find(|r| {
                r.root == root
                    && r.stage == DiagnosticStage::Commit
                    && r.outcome == DiagnosticOutcome::Succeeded
            })
            .expect("child commit links to pre-restart root");
        assert!(child.parent.is_some());
        assert_eq!(child.parent, Some(mapper_work));
        assert!(child.work.is_some());
        assert!(child.claim.is_some());
        assert!(linked.iter().any(|r| {
            r.root == root
                && r.stage == DiagnosticStage::ExternalAttempt
                && r.outcome == DiagnosticOutcome::Succeeded
                && r.work.is_some()
        }));
        assert!(
            linked
                .iter()
                .any(|r| r.stage == DiagnosticStage::DeliveryStart
                    && r.outcome == DiagnosticOutcome::Succeeded)
        );
        assert!(
            linked
                .iter()
                .any(|r| r.stage == DiagnosticStage::DeliveryFinish
                    && r.outcome == DiagnosticOutcome::Succeeded)
        );
        assert!(
            linked
                .iter()
                .any(|r| r.stage == DiagnosticStage::ExternalAttempt
                    && r.outcome == DiagnosticOutcome::Succeeded
                    && r.stage_elapsed_ns.is_some())
        );
        assert!(
            native
                .storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done)
        );
        assert_eq!(
            runtime
                .read::<Mirror>(&owner(), "mirror")
                .await
                .unwrap()
                .revision,
            2
        );
        assert_redacted(&origin);
        assert_redacted(&mapping);
        assert_redacted(&linked);
        runtime.shutdown().await.unwrap();
    }
}

// Catches leaking authorization subjects, Resource payload or durable retry identity from denied replay.
#[tokio::test]
async fn unauthorized_receipt_disclosure_emits_category_without_private_data() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 26, 1);
        let runtime = build(
            Runtime::builder().diagnostics(sink).resource(
                Private::definition()
                    .allow_all_fields()
                    .policy(|actor, _, row| {
                        actor.principal_kind() == PrincipalKind::Service
                            || actor.subject == row.owner
                    }),
            ),
            &native,
        );
        let alice = Actor::trusted("ISSUER_SECRET", "PRIVATE_OWNER_SECRET");
        let original = Command::create(
            "RESOURCE_ID_SECRET",
            Private {
                owner: "PRIVATE_OWNER_SECRET".into(),
                secret: "PRIVATE_VALUE_SECRET".into(),
            },
        )
        .idempotency("IDEMPOTENCY_SECRET");
        runtime.execute(&alice, original.clone()).await.unwrap();
        let command = Command::<Private>::patch(
            "RESOURCE_ID_SECRET",
            Patch::new().set(Private::owner_field(), "next-owner".to_owned()),
        )
        .at_revision(1)
        .idempotency("change-owner");
        runtime.execute(&service(), command).await.unwrap();
        drain(&mut reader);
        assert!(matches!(
            runtime.execute(&alice, original).await,
            Err(Error::Denied)
        ));
        let denied = drain(&mut reader);
        assert!(
            denied
                .iter()
                .any(|r| r.stage == DiagnosticStage::Authorization
                    && r.outcome == DiagnosticOutcome::Denied)
        );
        assert!(!denied.iter().any(
            |r| r.stage == DiagnosticStage::Execution && r.outcome == DiagnosticOutcome::Denied
        ));
        assert_redacted(&denied);
        assert_eq!(
            native
                .storage()
                .journal(Private::KIND, None, 16, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
        runtime.shutdown().await.unwrap();
    }
}

// Catches treating host stream identity as causal identity or retaining the same tokens after key rotation.
#[tokio::test]
async fn host_session_changes_preserve_causal_links_and_key_rotation_changes_them() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let mut original = None;
        for (key, session) in [(27, 1), (27, 2), (28, 3)] {
            let native = Native::open(redb, &scratch.database());
            let (sink, mut reader) = diagnostics(512, key, session);
            let runtime = build(builder(sink), &native);
            assert_eq!(
                runtime
                    .execute(&owner(), source("IDEMPOTENCY_SECRET"))
                    .await
                    .unwrap()
                    .revision,
                1
            );
            let records = drain(&mut reader);
            let observed = records
                .iter()
                .find(|r| {
                    (r.stage == DiagnosticStage::Commit
                        && r.outcome == DiagnosticOutcome::Succeeded)
                        || (r.stage == DiagnosticStage::Receipt
                            && r.outcome == DiagnosticOutcome::Replay)
                })
                .expect("confirmed commit or replay diagnostic");
            assert_eq!(observed.session, [session; 16]);
            match original {
                None => original = Some((observed.operation, observed.root)),
                Some((operation, root)) if key == 27 => {
                    assert_eq!(observed.operation, operation);
                    assert_eq!(observed.root, root);
                }
                Some((operation, root)) => {
                    assert_ne!(observed.operation, operation);
                    assert_ne!(observed.root, root);
                }
            }
            assert_eq!(
                native
                    .storage()
                    .journal(Source::KIND, None, 16, 100_000)
                    .unwrap()
                    .events
                    .len(),
                1
            );
            assert_redacted(&records);
            runtime.shutdown().await.unwrap();
        }
    }
}

static CALLBACK_ENTERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CALLBACK_RELEASED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const HELD_ACTION: Action<Source, bool> = Action::new("held", |state, enabled| {
    CALLBACK_ENTERED.store(true, Ordering::SeqCst);
    while !CALLBACK_RELEASED.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1));
    }
    state.enabled = enabled;
    Ok(vec![])
});
struct ReleaseCallback;
impl Drop for ReleaseCallback {
    fn drop(&mut self) {
        CALLBACK_RELEASED.store(true, Ordering::SeqCst);
    }
}

// Catches cancelling supervised execution with its caller or emitting fictitious terminal completion before native work finishes.
#[tokio::test]
async fn cancelled_caller_retains_owned_action_and_true_terminal_diagnostics() {
    for redb in [false, true] {
        CALLBACK_ENTERED.store(false, Ordering::SeqCst);
        CALLBACK_RELEASED.store(false, Ordering::SeqCst);
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut held_reader) = diagnostics(512, 29, 2);
        let runtime = build(
            Runtime::builder().diagnostics(sink).resource(
                Source::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true)
                    .action(HELD_ACTION),
            ),
            &native,
        );
        runtime
            .execute(
                &owner(),
                Command::create(
                    "RESOURCE_ID_SECRET",
                    Source {
                        enabled: false,
                        title: "RESOURCE_PAYLOAD_SECRET".into(),
                    },
                )
                .idempotency("seed-held"),
            )
            .await
            .unwrap();
        drain(&mut held_reader);
        let release_on_failure = ReleaseCallback;
        let worker = runtime.clone();
        let caller = tokio::spawn(async move {
            worker
                .execute(
                    &owner(),
                    Command::action("RESOURCE_ID_SECRET", HELD_ACTION, true)
                        .at_revision(1)
                        .idempotency("IDEMPOTENCY_SECRET"),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while !CALLBACK_ENTERED.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("real action callback must enter");
        caller.abort();
        match caller.await {
            Err(error) => assert!(error.is_cancelled()),
            Ok(_) => panic!("caller unexpectedly completed before callback release"),
        }
        assert_eq!(runtime.available_capacity(), Limits::default().actions - 1);
        let unfinished = drain(&mut held_reader);
        assert!(!unfinished.iter().any(|r| {
            r.stage == DiagnosticStage::Commit && r.outcome == DiagnosticOutcome::Succeeded
        }));
        drop(release_on_failure);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if runtime
                    .read::<Source>(&owner(), "RESOURCE_ID_SECRET")
                    .await
                    .unwrap()
                    .revision
                    == 2
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("owned action must finish after caller cancellation");
        runtime.shutdown().await.unwrap();
        let finished = drain(&mut held_reader);
        assert!(finished.iter().any(|r| {
            r.stage == DiagnosticStage::Commit && r.outcome == DiagnosticOutcome::Succeeded
        }));
        assert_redacted(&finished);
        assert_eq!(
            native
                .storage()
                .journal(Source::KIND, None, 16, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
    }
}

// Catches waiting for exporter progress or allowing exporter panic to replace a business outcome.
#[tokio::test]
async fn sleeping_and_panicked_host_exporters_do_not_own_runtime_shutdown() {
    for redb in [false, true] {
        for panics in [false, true] {
            let scratch = Scratch::new();
            let native = Native::open(redb, &scratch.database());
            let (sink, mut reader) = diagnostics(1, 30, 1);
            let runtime = build(builder(sink), &native);
            let (started, entered) = tokio::sync::oneshot::channel();
            let exporter = tokio::spawn(async move {
                reader.recv().await.expect("real diagnostic record");
                let _ = started.send(());
                if panics {
                    panic!("host exporter failure");
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
                reader
            });
            assert_eq!(
                tokio::time::timeout(
                    Duration::from_secs(5),
                    runtime.execute(&owner(), source("IDEMPOTENCY_SECRET")),
                )
                .await
                .expect("exporter cannot block action")
                .unwrap()
                .revision,
                1
            );
            tokio::time::timeout(Duration::from_secs(5), entered)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                runtime
                    .execute(&owner(), source("IDEMPOTENCY_SECRET"))
                    .await
                    .unwrap()
                    .revision,
                1
            );
            tokio::time::timeout(Duration::from_secs(5), runtime.shutdown())
                .await
                .expect("exporter cannot block shutdown")
                .unwrap();
            if panics {
                match exporter.await {
                    Err(error) => assert!(error.is_panic()),
                    Ok(_) => panic!("panicked exporter returned normally"),
                }
            } else {
                exporter.abort();
                match exporter.await {
                    Err(error) => assert!(error.is_cancelled()),
                    Ok(_) => panic!("sleeping exporter returned before cancellation"),
                }
            }
            assert_eq!(
                native
                    .storage()
                    .journal(Source::KIND, None, 16, 100_000)
                    .unwrap()
                    .events
                    .len(),
                1
            );
        }
    }
}

// Catches silent invalid diagnostic configuration that can allocate unbounded queues or omit host stream identity.
#[test]
fn invalid_capacity_or_missing_host_session_is_rejected_before_intake() {
    for capacity in [0, 4097, usize::MAX] {
        assert!(
            Diagnostics::bounded(
                DiagnosticOptions::new(DiagnosticKey::new([31; 32]), [1; 16]).capacity(capacity),
            )
            .is_err()
        );
    }
    assert!(
        Diagnostics::bounded(DiagnosticOptions::new(
            DiagnosticKey::new([31; 32]),
            [0; 16]
        ),)
        .is_err()
    );
}

// A persisted delivery status is not evidence that an ambiguous external effect succeeded.
#[tokio::test]
async fn unknown_external_attempt_is_held_without_success_or_resend_after_reopen() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 29, 1);
        let attempts = Arc::new(AtomicU64::new(0));
        let send_attempts = attempts.clone();
        let runtime = build(
            builder(sink)
                .resource(
                    Mirror::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true)
                        .action(SET_MIRROR),
                )
                .channel_with(
                    MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry),
                    service(),
                    move |_| {
                        send_attempts.fetch_add(1, Ordering::SeqCst);
                        async { DeliveryOutcome::Unknown }
                    },
                ),
            &native,
        );
        runtime
            .execute(
                &owner(),
                Command::create("mirror", Mirror { enabled: false }).idempotency("seed-mirror"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &owner(),
                Command::action("mirror", SET_MIRROR, true)
                    .at_revision(1)
                    .idempotency("uncertain-mail"),
            )
            .await
            .unwrap();
        drain(&mut reader);
        assert_eq!(runtime.process_work(8).await.unwrap(), 1);
        let records = drain(&mut reader);
        let external = records
            .iter()
            .find(|record| {
                record.stage == DiagnosticStage::ExternalAttempt
                    && record.outcome == DiagnosticOutcome::Unknown
            })
            .expect("external uncertainty is explicit");
        assert!(external.stage_elapsed_ns.is_some());
        assert!(records.iter().any(|record| record.work == external.work
            && record.stage == DiagnosticStage::DeliveryFinish
            && record.outcome == DiagnosticOutcome::Succeeded));
        assert!(!records.iter().any(|record| record.work == external.work
            && matches!(
                record.stage,
                DiagnosticStage::ExternalAttempt | DiagnosticStage::Work
            )
            && record.outcome == DiagnosticOutcome::Succeeded));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(
            native.storage().reaction_records().unwrap()[0].state,
            WorkState::AwaitingReconciliation
        );
        assert_redacted(&records);
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(native);
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 29, 2);
        let send_attempts = attempts.clone();
        let runtime = build(
            builder(sink)
                .resource(
                    Mirror::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true)
                        .action(SET_MIRROR),
                )
                .channel_with(
                    MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry),
                    service(),
                    move |_| {
                        send_attempts.fetch_add(1, Ordering::SeqCst);
                        async { DeliveryOutcome::Accepted }
                    },
                ),
            &native,
        );
        assert_eq!(runtime.process_work(8).await.unwrap(), 0);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert!(
            !drain(&mut reader)
                .iter()
                .any(|record| record.stage == DiagnosticStage::ExternalAttempt)
        );
        assert_eq!(
            native.storage().reaction_records().unwrap()[0].state,
            WorkState::AwaitingReconciliation
        );
        runtime.shutdown().await.unwrap();
    }
}

// Run only under the coordinator's native lease. This is a bounded workload, not a production SLO.
#[tokio::test]
#[ignore = "bounded diagnostic overhead characterization"]
async fn characterize_diagnostics_command_overhead() {
    fn percentile(values: &mut [u64], numerator: usize) -> u64 {
        values.sort_unstable();
        values[(values.len() - 1) * numerator / 100]
    }
    for redb in [false, true] {
        for round in 0..3 {
            for mode in ["disabled", "drained", "undrained"] {
                let scratch = Scratch::new();
                let native = Native::open(redb, &scratch.database());
                let mut reader = None;
                let mut configured = Runtime::builder().resource(
                    Source::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true),
                );
                if mode != "disabled" {
                    let (sink, observed) =
                        diagnostics(if mode == "undrained" { 1 } else { 512 }, 31, 1);
                    configured = configured.diagnostics(sink);
                    reader = Some(observed);
                }
                let runtime = build(configured, &native);
                let mut creates = Vec::with_capacity(64);
                let mut replays = Vec::with_capacity(512);
                let mut exported = 0usize;
                let start = std::time::Instant::now();
                for index in 0..64 {
                    let command = Command::create(
                        &format!("item-{index}"),
                        Source {
                            enabled: true,
                            title: "bounded fixture".into(),
                        },
                    )
                    .idempotency(&format!("create-{index}"));
                    let started = std::time::Instant::now();
                    assert_eq!(
                        runtime.execute(&owner(), command).await.unwrap().revision,
                        1
                    );
                    if mode == "drained" {
                        exported += drain(reader.as_mut().unwrap()).len();
                    }
                    creates.push(u64::try_from(started.elapsed().as_nanos()).unwrap());
                }
                for index in 0..512 {
                    let id = index % 64;
                    let command = Command::create(
                        &format!("item-{id}"),
                        Source {
                            enabled: true,
                            title: "bounded fixture".into(),
                        },
                    )
                    .idempotency(&format!("create-{id}"));
                    let started = std::time::Instant::now();
                    assert_eq!(
                        runtime.execute(&owner(), command).await.unwrap().revision,
                        1
                    );
                    if mode == "drained" {
                        exported += drain(reader.as_mut().unwrap()).len();
                    }
                    replays.push(u64::try_from(started.elapsed().as_nanos()).unwrap());
                }
                let wall_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
                let stats = reader.as_ref().map(DiagnosticReader::stats);
                println!(
                    "ROM_DIAGNOSTICS_OVERHEAD {}",
                    serde_json::json!({
                        "adapter": if redb { "redb" } else { "sqlite" }, "round": round, "mode": mode,
                        "create_count": creates.len(), "replay_count": replays.len(), "wall_ns": wall_ns,
                        "create_p50_ns": percentile(&mut creates, 50), "create_p95_ns": percentile(&mut creates, 95), "create_p99_ns": percentile(&mut creates, 99),
                        "replay_p50_ns": percentile(&mut replays, 50), "replay_p95_ns": percentile(&mut replays, 95), "replay_p99_ns": percentile(&mut replays, 99),
                        "exported": exported, "stats": stats,
                        "includes_host_drain": mode == "drained", "seed": "fixed-64-resources-512-replays-v1"
                    })
                );
                assert_eq!(
                    native
                        .storage()
                        .journal(Source::KIND, None, 128, 1_000_000)
                        .unwrap()
                        .events
                        .len(),
                    64
                );
                runtime.shutdown().await.unwrap();
            }
        }
    }
}

static SHUTDOWN_ENTERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SHUTDOWN_RELEASED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const SHUTDOWN_ACTION: Action<Source, bool> = Action::new("shutdown-held", |state, enabled| {
    SHUTDOWN_ENTERED.store(true, Ordering::SeqCst);
    while !SHUTDOWN_RELEASED.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1));
    }
    state.enabled = enabled;
    Ok(vec![])
});
struct ReleaseShutdown;
impl Drop for ReleaseShutdown {
    fn drop(&mut self) {
        SHUTDOWN_RELEASED.store(true, Ordering::SeqCst);
    }
}

// Cancelling a drain waiter must not publish completion or abandon accepted native work.
#[tokio::test]
async fn cancelled_shutdown_waiter_preserves_status_and_observed_terminal_boundary() {
    for redb in [false, true] {
        SHUTDOWN_ENTERED.store(false, Ordering::SeqCst);
        SHUTDOWN_RELEASED.store(false, Ordering::SeqCst);
        let release = ReleaseShutdown;
        let scratch = Scratch::new();
        let native = Native::open(redb, &scratch.database());
        let (sink, mut reader) = diagnostics(512, 33, 1);
        let runtime = build(
            Runtime::builder().diagnostics(sink).resource(
                Source::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true)
                    .action(SHUTDOWN_ACTION),
            ),
            &native,
        );
        runtime
            .execute(&owner(), source("shutdown-seed"))
            .await
            .unwrap();
        drain(&mut reader);
        let worker = runtime.clone();
        let caller = tokio::spawn(async move {
            worker
                .execute(
                    &owner(),
                    Command::action("RESOURCE_ID_SECRET", SHUTDOWN_ACTION, false)
                        .at_revision(1)
                        .idempotency("shutdown-action"),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while !SHUTDOWN_ENTERED.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        let draining = runtime.clone();
        let waiter = tokio::spawn(async move { draining.shutdown().await });
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime.status().unwrap().intake != IntakeState::Draining {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        let started = drain(&mut reader);
        assert!(
            started
                .iter()
                .any(|record| record.stage == DiagnosticStage::Shutdown
                    && record.outcome == DiagnosticOutcome::Started)
        );
        assert!(
            !started
                .iter()
                .any(|record| record.stage == DiagnosticStage::Shutdown
                    && record.outcome == DiagnosticOutcome::Succeeded)
        );
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert_eq!(runtime.status().unwrap().intake, IntakeState::Draining);
        caller.abort();
        assert!(matches!(caller.await, Err(error) if error.is_cancelled()));
        drop(release);
        tokio::time::timeout(Duration::from_secs(5), runtime.shutdown())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(runtime.status().unwrap().intake, IntakeState::Stopped);
        let finished = drain(&mut reader);
        assert_eq!(
            finished
                .iter()
                .filter(|record| record.stage == DiagnosticStage::Shutdown
                    && record.outcome == DiagnosticOutcome::Succeeded)
                .count(),
            1
        );
        assert_eq!(
            native
                .storage()
                .journal(Source::KIND, None, 16, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
        assert_redacted(&started);
        assert_redacted(&finished);
    }
}
