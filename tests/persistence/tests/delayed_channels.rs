//! Shared actual-adapter proof for frozen delayed notification admission and restart.
use rom::*;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

#[derive(Clone, Resource)]
#[resource(name = "delayed-notices")]
struct Notice {
    count: u64,
}
const MAIL: Channel<String> = Channel::new("delayed-mail", 1);
const NOTIFY: Action<Notice, u64> = Action::new("notify", |state, not_before| {
    state.count += 1;
    Ok(vec![
        MAIL.intent_at(format!("notice {}", state.count), not_before),
    ])
});
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn owner() -> Actor {
    Actor::trusted("host", "owner")
}
fn service() -> Actor {
    Actor::trusted("host", "mailer").with_kind(PrincipalKind::Service)
}
fn path(redb: bool, scenario: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = std::env::temp_dir().join(format!(
        "rom-delayed-{}-{scenario}-{redb}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&parent).unwrap();
    parent.join("database")
}
fn db(redb: bool, path: &Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn build(store: Arc<dyn Storage>, clock: Arc<TestClock>, calls: Arc<AtomicUsize>) -> Runtime {
    build_profile(store, clock, calls, DeliveryProfile::AtLeastOnce)
}
fn build_profile(
    store: Arc<dyn Storage>,
    clock: Arc<TestClock>,
    calls: Arc<AtomicUsize>,
    profile: DeliveryProfile,
) -> Runtime {
    Runtime::builder()
        .clock(clock)
        .resource(
            Notice::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(NOTIFY),
        )
        .channel_with(
            MAIL.delivery_profile(profile),
            service(),
            move |delivery: Delivery<String>| {
                assert_eq!(delivery.payload, "notice 1");
                calls.fetch_add(1, Ordering::SeqCst);
                async { DeliveryOutcome::Accepted }
            },
        )
        .build(store, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
async fn seed(rt: &Runtime) {
    rt.execute(
        &owner(),
        Command::create("one", Notice { count: 0 }).idempotency("seed"),
    )
    .await
    .unwrap();
    rt.execute(
        &owner(),
        Command::action("one", NOTIFY, 20)
            .at_revision(1)
            .idempotency("delay"),
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn delayed_commit_replay_restart_and_timing_mismatch_on_both_adapters() {
    for redb in [false, true] {
        let path = path(redb, "replay");
        let store = db(redb, &path);
        let clock = Arc::new(TestClock(AtomicU64::new(10)));
        let calls = Arc::new(AtomicUsize::new(0));
        let rt = build(store.clone(), clock.clone(), calls.clone());
        seed(&rt).await;
        let frozen = store.reaction_records().unwrap();
        assert_eq!(frozen.len(), 1);
        assert_eq!(frozen[0].pending.not_before, Some(20));
        assert_eq!(frozen[0].due, 20);
        let events = store.journal(Notice::KIND, None, 20, 100_000).unwrap();
        rt.execute(
            &owner(),
            Command::action("one", NOTIFY, 20)
                .at_revision(1)
                .idempotency("delay"),
        )
        .await
        .unwrap();
        assert!(matches!(
            rt.execute(
                &owner(),
                Command::action("one", NOTIFY, 21)
                    .at_revision(1)
                    .idempotency("delay")
            )
            .await,
            Err(Error::IdentityMismatch)
        ));
        assert_eq!(store.reaction_records().unwrap(), frozen);
        assert_eq!(
            store.journal(Notice::KIND, None, 20, 100_000).unwrap(),
            events
        );
        clock.0.store(19, Ordering::SeqCst);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        assert_eq!(store.reaction_records().unwrap(), frozen);
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = db(redb, &path);
        let rt = build(store.clone(), clock.clone(), calls.clone());
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        clock.0.store(20, Ordering::SeqCst);
        assert_eq!(rt.process_work(8).await.unwrap(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(rt.process_work(8).await.unwrap(), 0);
        let records = store.reaction_records().unwrap();
        assert_eq!(records[0].state, WorkState::Done);
        assert_eq!(records[0].attempts, 1);
        assert_eq!(records[0].pending.not_before, Some(20));
        rt.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn existing_worker_resumes_after_restart_without_application_scheduler() {
    for redb in [false, true] {
        let path = path(redb, "worker");
        let store = db(redb, &path);
        let clock = Arc::new(TestClock(AtomicU64::new(10)));
        let calls = Arc::new(AtomicUsize::new(0));
        let rt = build(store.clone(), clock.clone(), calls.clone());
        seed(&rt).await;
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = db(redb, &path);
        let (sent, received) = tokio::sync::oneshot::channel();
        let sent = Arc::new(std::sync::Mutex::new(Some(sent)));
        let counter = calls.clone();
        let rt = Runtime::builder()
            .clock(clock.clone())
            .resource(
                Notice::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true)
                    .action(NOTIFY),
            )
            .channel(MAIL, service(), move |_: Delivery<String>| {
                counter.fetch_add(1, Ordering::SeqCst);
                if let Some(sent) = sent.lock().unwrap().take() {
                    sent.send(()).unwrap();
                }
                async { DeliveryOutcome::Accepted }
            })
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let worker = rt.start_work().unwrap();
        clock.0.store(19, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(store.reaction_records().unwrap()[0].attempts, 0);
        clock.0.store(20, Ordering::SeqCst);
        tokio::time::timeout(std::time::Duration::from_secs(2), received)
            .await
            .unwrap()
            .unwrap();
        rt.shutdown().await.unwrap();
        worker.join().await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(store.reaction_records().unwrap()[0].state, WorkState::Done);
    }
}
#[tokio::test]
async fn delayed_delivery_rechecks_authority_and_channel_version_at_eligibility() {
    for redb in [false, true] {
        for changed_version in [false, true] {
            let path = path(redb, if changed_version { "version" } else { "denied" });
            let store = db(redb, &path);
            let clock = Arc::new(TestClock(AtomicU64::new(10)));
            let calls = Arc::new(AtomicUsize::new(0));
            let rt = build(store.clone(), clock.clone(), calls.clone());
            seed(&rt).await;
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let store = db(redb, &path);
            let counter = calls.clone();
            let channel = if changed_version {
                Channel::new("delayed-mail", 2)
            } else {
                MAIL
            };
            let rt = Runtime::builder()
                .clock(clock.clone())
                .resource(
                    Notice::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true)
                        .action(NOTIFY),
                )
                .channel(channel, service(), move |_: Delivery<String>| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                })
                .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            if !changed_version {
                rt.revoke(&service());
            }
            clock.0.store(19, Ordering::SeqCst);
            assert_eq!(rt.process_work(8).await.unwrap(), 0);
            assert_eq!(store.reaction_records().unwrap()[0].attempts, 0);
            clock.0.store(20, Ordering::SeqCst);
            assert_eq!(rt.process_work(8).await.unwrap(), 1);
            let record = &store.reaction_records().unwrap()[0];
            assert_eq!(
                record.state,
                WorkState::Stopped(if changed_version {
                    StopReason::DefinitionChanged
                } else {
                    StopReason::Denied
                })
            );
            assert_eq!(record.delivery, None);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            rt.shutdown().await.unwrap();
        }
    }
}
#[tokio::test]
async fn archive_restore_keeps_delayed_floor_and_unknown_reconciliation_on_both_adapters() {
    for redb in [false, true] {
        for reconcile in [false, true] {
            let source = path(
                redb,
                if reconcile {
                    "restore-reconcile"
                } else {
                    "restore-floor"
                },
            );
            let archive = source.with_extension("rombk");
            let destination = source.with_extension("restored");
            let store = db(redb, &source);
            let clock = Arc::new(TestClock(AtomicU64::new(10)));
            let calls = Arc::new(AtomicUsize::new(0));
            let profile = if reconcile {
                DeliveryProfile::ReconcileBeforeRetry
            } else {
                DeliveryProfile::AtLeastOnce
            };
            let rt = Runtime::builder()
                .clock(clock.clone())
                .resource(
                    Notice::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true)
                        .action(NOTIFY),
                )
                .channel_with(
                    MAIL.delivery_profile(profile),
                    service(),
                    |_: Delivery<String>| async { DeliveryOutcome::Accepted },
                )
                .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            seed(&rt).await;
            let WorkResult::Claimed(old) = store
                .reaction_update(WorkUpdate::Claim { now: 20 })
                .unwrap()
            else {
                panic!()
            };
            if reconcile {
                store
                    .reaction_update(WorkUpdate::DeliveryStarted {
                        claim: old.key(),
                        now: 20,
                    })
                    .unwrap();
            }
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let restored: Arc<dyn Storage> = if redb {
                let source = rom_redb::Redb::open(&source).unwrap();
                source
                    .backup_to(&archive, rom_backup::BackupLimits::default())
                    .unwrap();
                Arc::new(
                    rom_redb::Redb::restore_from(
                        &archive,
                        &destination,
                        rom_backup::BackupLimits::default(),
                    )
                    .unwrap(),
                )
            } else {
                let source = rom_sqlite::Sqlite::open(&source).unwrap();
                source
                    .backup_to(&archive, rom_backup::BackupLimits::default())
                    .unwrap();
                Arc::new(
                    rom_sqlite::Sqlite::restore_from(
                        &archive,
                        &destination,
                        rom_backup::BackupLimits::default(),
                    )
                    .unwrap(),
                )
            };
            let record = &restored.reaction_records().unwrap()[0];
            assert_eq!(record.pending.not_before, Some(20));
            assert_eq!(record.due, 20);
            assert_eq!(record.attempts, 1);
            assert!(record.generation > old.work.generation);
            assert_eq!(
                restored.reaction_update(WorkUpdate::DeliveryFinished {
                    claim: old.key(),
                    now: 21,
                    outcome: DeliveryOutcome::Accepted
                }),
                Err(Error::Conflict)
            );
            let rt = build_profile(
                restored.clone(),
                clock.clone(),
                calls.clone(),
                if reconcile {
                    DeliveryProfile::ReconcileBeforeRetry
                } else {
                    DeliveryProfile::AtLeastOnce
                },
            );
            clock.0.store(19, Ordering::SeqCst);
            assert_eq!(rt.process_work(8).await.unwrap(), 0);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            clock.0.store(20, Ordering::SeqCst);
            if reconcile {
                assert_eq!(rt.process_work(8).await.unwrap(), 0);
                assert_eq!(
                    restored.reaction_records().unwrap()[0].state,
                    WorkState::AwaitingReconciliation
                );
                assert_eq!(calls.load(Ordering::SeqCst), 0);
            } else {
                assert_eq!(rt.process_work(8).await.unwrap(), 1);
                assert_eq!(calls.load(Ordering::SeqCst), 1);
            }
            rt.shutdown().await.unwrap();
        }
    }
}
