use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
#[derive(Clone, Resource)]
#[resource(name = "notices")]
struct Notice {
    count: u64,
}
const MAIL: Channel<String> = Channel::new("mail", 1);
const NOTIFY: Action<Notice, ()> = Action::new("notify", |state, ()| {
    state.count += 1;
    Ok(vec![MAIL.intent(format!("notice {}", state.count))])
});
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn service() -> Actor {
    Actor::trusted("host", "notifier").with_kind(PrincipalKind::Service)
}
fn actor() -> Actor {
    Actor::trusted("host", "owner")
}
fn builder(clock: Arc<TestClock>) -> Builder {
    Runtime::builder().clock(clock).resource(
        Notice::definition()
            .allow_all_fields()
            .policy(|_, _, _| true)
            .action(NOTIFY),
    )
}
fn db(redb: bool, path: &std::path::Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn path(name: &str, redb: bool) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("rom-channel-{}-{name}-{redb}", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}
fn build(b: Builder, store: Arc<dyn Storage>) -> Runtime {
    b.build(store, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
async fn enqueue(runtime: &Runtime) {
    runtime
        .execute(
            &actor(),
            Command::create("one", Notice { count: 0 }).idempotency("seed"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor(),
            Command::action("one", NOTIFY, ())
                .at_revision(1)
                .idempotency("notify"),
        )
        .await
        .unwrap();
}
#[tokio::test]
async fn ordinary_action_records_typed_notification_atomically_without_sending() {
    for redb in [false, true] {
        let p = path("record", redb);
        let store = db(redb, &p);
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let rt = build(
            builder(Arc::new(TestClock(AtomicU64::new(0)))).channel(
                MAIL,
                service(),
                move |_: Delivery<String>| {
                    count.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                },
            ),
            store.clone(),
        );
        enqueue(&rt).await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = db(redb, &p);
        let records = store.reaction_records().unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(
            records[0].pending.payload,
            WorkPayload::Notification { .. }
        ));
        assert_eq!(records[0].state, WorkState::Pending);
        assert_eq!(
            store
                .load(&Key {
                    kind: Notice::KIND.into(),
                    id: "one".into()
                })
                .unwrap()
                .unwrap()
                .revision,
            2
        );
        drop(store);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn shared_worker_invokes_typed_function_and_records_acceptance() {
    for redb in [false, true] {
        let p = path("accepted", redb);
        let store = db(redb, &p);
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let rt = build(
            builder(Arc::new(TestClock(AtomicU64::new(0)))).channel(
                MAIL,
                service(),
                move |delivery: Delivery<String>| {
                    assert_eq!(delivery.payload, "notice 1");
                    assert_eq!(delivery.attempt, 1);
                    count.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                },
            ),
            store.clone(),
        );
        enqueue(&rt).await;
        assert_eq!(rt.process_reactions(8).await.unwrap(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let records = store.reaction_records().unwrap();
        assert_eq!(records[0].state, WorkState::Done);
        assert_eq!(records[0].delivery, Some(DeliveryOutcome::Accepted));
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn retry_and_permanent_outcomes_are_durable_and_bounded() {
    for redb in [false, true] {
        for outcome in [
            DeliveryOutcome::Retryable,
            DeliveryOutcome::Permanent,
            DeliveryOutcome::Unknown,
        ] {
            let p = path(&format!("outcome-{outcome:?}"), redb);
            let store = db(redb, &p);
            let clock = Arc::new(TestClock(AtomicU64::new(0)));
            let result = outcome.clone();
            let rt = build(
                builder(clock.clone())
                    .reaction_limits(ReactionLimits {
                        max_attempts: 2,
                        ..Default::default()
                    })
                    .channel(MAIL, service(), move |_: Delivery<String>| {
                        let result = result.clone();
                        async move { result }
                    }),
                store.clone(),
            );
            enqueue(&rt).await;
            assert_eq!(rt.process_work(8).await.unwrap(), 1);
            let record = &store.reaction_records().unwrap()[0];
            assert_eq!(record.delivery, Some(outcome.clone()));
            if outcome == DeliveryOutcome::Permanent {
                assert_eq!(
                    record.state,
                    WorkState::Stopped(StopReason::DeliveryPermanent)
                );
            } else {
                assert_eq!(record.state, WorkState::Pending);
                assert_eq!(rt.process_work(8).await.unwrap(), 0);
                clock.0.store(1, Ordering::SeqCst);
                assert_eq!(rt.process_work(8).await.unwrap(), 1);
                let records = store.reaction_records().unwrap();
                assert_eq!(records[0].attempts, 2);
                assert_eq!(records[0].state, WorkState::Stopped(StopReason::Attempts));
                clock.0.store(100, Ordering::SeqCst);
                assert_eq!(rt.process_work(8).await.unwrap(), 0);
            }
            assert_eq!(
                rt.read::<Notice>(&actor(), "one")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .count,
                1
            );
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[tokio::test]
async fn unknown_channel_version_and_payload_reject_upstream_commit() {
    for redb in [false, true] {
        for mode in 0..3 {
            const BAD_VERSION: Action<Notice, ()> = Action::new("bad-version", |state, ()| {
                state.count += 1;
                Ok(vec![
                    Channel::<String>::new("mail", 2).intent("notice".into()),
                ])
            });
            const BAD_PAYLOAD: Action<Notice, ()> = Action::new("bad-payload", |state, ()| {
                state.count += 1;
                Ok(vec![Channel::<bool>::new("mail", 1).intent(true)])
            });
            let p = path(&format!("invalid-{mode}"), redb);
            let store = db(redb, &p);
            let definition = Notice::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(NOTIFY)
                .action(BAD_VERSION)
                .action(BAD_PAYLOAD);
            let mut b = Runtime::builder().resource(definition);
            if mode != 0 {
                b = b.channel(MAIL, service(), |_: Delivery<String>| async {
                    DeliveryOutcome::Accepted
                });
            }
            let rt = build(b, store.clone());
            rt.execute(
                &actor(),
                Command::create("one", Notice { count: 0 }).idempotency("seed"),
            )
            .await
            .unwrap();
            let action = match mode {
                0 => NOTIFY,
                1 => BAD_VERSION,
                _ => BAD_PAYLOAD,
            };
            assert!(
                rt.execute(
                    &actor(),
                    Command::action("one", action, ())
                        .at_revision(1)
                        .idempotency("bad")
                )
                .await
                .is_err()
            );
            assert_eq!(
                rt.read::<Notice>(&actor(), "one").await.unwrap().revision,
                1
            );
            assert!(store.reaction_records().unwrap().is_empty());
            assert_eq!(
                store
                    .journal(Notice::KIND, None, 16, 16_384)
                    .unwrap()
                    .events
                    .len(),
                1
            );
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[tokio::test]
async fn revoked_channel_service_never_sees_payload() {
    for redb in [false, true] {
        let p = path("revoked", redb);
        let store = db(redb, &p);
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let rt = build(
            builder(Arc::new(TestClock(AtomicU64::new(0)))).channel(
                MAIL,
                service(),
                move |_: Delivery<String>| {
                    count.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                },
            ),
            store.clone(),
        );
        enqueue(&rt).await;
        rt.revoke(&service());
        rt.process_work(8).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            store.reaction_records().unwrap()[0].state,
            WorkState::Stopped(StopReason::Denied)
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn effect_then_lost_ack_reopens_with_stable_key_and_receiver_owned_dedup() {
    for redb in [false, true] {
        for dedup in [false, true] {
            let p = path(&format!("lost-{dedup}"), redb);
            let clock = Arc::new(TestClock(AtomicU64::new(0)));
            let receiver = Arc::new(std::sync::Mutex::new((Vec::<String>::new(), 0usize)));
            let mut prior_id = None;
            for attempt in 0..2 {
                let store = db(redb, &p);
                let received = receiver.clone();
                let rt = build(
                    builder(clock.clone()).channel(
                        MAIL,
                        service(),
                        move |delivery: Delivery<String>| {
                            let mut state = received.lock().unwrap();
                            if !dedup || !state.0.contains(&delivery.id) {
                                state.1 += 1;
                            }
                            state.0.push(delivery.id);
                            async move {
                                if attempt == 0 {
                                    DeliveryOutcome::Unknown
                                } else {
                                    DeliveryOutcome::Accepted
                                }
                            }
                        },
                    ),
                    store.clone(),
                );
                if attempt == 0 {
                    enqueue(&rt).await;
                }
                assert_eq!(rt.process_work(8).await.unwrap(), 1);
                let record = &store.reaction_records().unwrap()[0];
                if let Some(id) = &prior_id {
                    assert_eq!(&record.pending.id, id);
                } else {
                    prior_id = Some(record.pending.id.clone());
                }
                assert_eq!(record.attempts, attempt + 1);
                assert_eq!(
                    record.state,
                    if attempt == 0 {
                        WorkState::Pending
                    } else {
                        WorkState::Done
                    }
                );
                rt.shutdown().await.unwrap();
                drop(rt);
                drop(store);
                clock.0.store(2, Ordering::SeqCst);
            }
            let state = receiver.lock().unwrap();
            assert_eq!(state.0.len(), 2);
            assert_eq!(state.0[0], state.0[1]);
            assert_eq!(state.1, if dedup { 1 } else { 2 });
            drop(state);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[tokio::test]
async fn timeout_is_uncertain_and_panics_are_inspectable() {
    for redb in [false, true] {
        for panic in [false, true] {
            let p = path(&format!("timeout-{panic}"), redb);
            let store = db(redb, &p);
            let rt = build(
                builder(Arc::new(TestClock(AtomicU64::new(0))))
                    .delivery_timeout(std::time::Duration::from_millis(20))
                    .reaction_limits(ReactionLimits {
                        max_attempts: 1,
                        ..Default::default()
                    })
                    .channel(MAIL, service(), move |_: Delivery<String>| async move {
                        if panic {
                            panic!("test callback panic")
                        };
                        std::future::pending::<DeliveryOutcome>().await
                    }),
                store.clone(),
            );
            enqueue(&rt).await;
            rt.process_work(8).await.unwrap();
            let record = &store.reaction_records().unwrap()[0];
            assert_eq!(
                record.delivery,
                Some(if panic {
                    DeliveryOutcome::Panicked
                } else {
                    DeliveryOutcome::TimedOut
                })
            );
            assert_eq!(
                record.state,
                WorkState::Stopped(if panic {
                    StopReason::CallbackPanicked
                } else {
                    StopReason::Attempts
                })
            );
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[tokio::test]
async fn cancelled_observer_cannot_orphan_delivery_and_shutdown_drains() {
    let p = path("cancel", false);
    let store = db(false, &p);
    let started = Arc::new(tokio::sync::Notify::new());
    let notify = started.clone();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let wait = Arc::new(std::sync::Mutex::new(Some(wait)));
    let rt = build(
        builder(Arc::new(TestClock(AtomicU64::new(0)))).channel(
            MAIL,
            service(),
            move |_: Delivery<String>| {
                let wait = wait.lock().unwrap().take().unwrap();
                let notify = notify.clone();
                async move {
                    notify.notify_one();
                    wait.await.unwrap();
                    DeliveryOutcome::Accepted
                }
            },
        ),
        store.clone(),
    );
    enqueue(&rt).await;
    let task_rt = rt.clone();
    let observer = tokio::spawn(async move { task_rt.process_work(1).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    assert_eq!(
        store.reaction_records().unwrap()[0].delivery,
        Some(DeliveryOutcome::Unknown)
    );
    observer.abort();
    let task_rt = rt.clone();
    let shutdown = tokio::spawn(async move { task_rt.shutdown().await });
    tokio::task::yield_now().await;
    assert!(!shutdown.is_finished());
    release.send(()).unwrap();
    shutdown.await.unwrap().unwrap();
    assert_eq!(store.reaction_records().unwrap()[0].state, WorkState::Done);
    assert_eq!(rt.available_io_capacity(), 8);
    drop(rt);
    drop(store);
    let _ = std::fs::remove_file(p);
}
#[tokio::test]
async fn registration_changes_stop_old_work_instead_of_reinterpreting_payload() {
    for redb in [false, true] {
        let p = path("version-change", redb);
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let store = db(redb, &p);
        let rt = build(
            builder(clock.clone()).channel(MAIL, service(), |_: Delivery<String>| async {
                DeliveryOutcome::Accepted
            }),
            store.clone(),
        );
        enqueue(&rt).await;
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let store = db(redb, &p);
        let rt = build(
            builder(clock).channel(
                Channel::<String>::new("mail", 2),
                service(),
                |_: Delivery<String>| async { panic!("old payload must not reach new definition") },
            ),
            store.clone(),
        );
        rt.process_work(8).await.unwrap();
        assert_eq!(
            store.reaction_records().unwrap()[0].state,
            WorkState::Stopped(StopReason::DefinitionChanged)
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let _ = std::fs::remove_file(p);
    }
}
#[test]
fn stale_external_ack_cannot_finish_new_claim() {
    let mut ledger = WorkLedger::default();
    let limits = ReactionLimits {
        lease_seconds: 1,
        ..Default::default()
    };
    let pending = PendingWork {
        id: "delivery".into(),
        cause: Cause {
            root: "root".into(),
            parent: None,
            depth: 0,
            started_at: 0,
            path: vec![],
        },
        definition: "channel".into(),
        version: 1,
        service_key: "service".into(),
        payload: WorkPayload::Notification {
            source: Row {
                key: Key {
                    kind: "notices".into(),
                    id: "one".into(),
                },
                revision: 1,
                protected: Default::default(),
                value: Some(json!({"count":1})),
            },
            payload: json!("payload"),
        },
    };
    ledger.enqueue(&limits, vec![pending]).unwrap();
    let WorkResult::Claimed(first) = ledger.apply(WorkUpdate::Claim { now: 0 }).unwrap() else {
        panic!()
    };
    ledger
        .apply(WorkUpdate::DeliveryStarted {
            claim: first.key(),
            now: 0,
        })
        .unwrap();
    let WorkResult::Claimed(second) = ledger.apply(WorkUpdate::Claim { now: 1 }).unwrap() else {
        panic!()
    };
    assert_eq!(
        ledger.apply(WorkUpdate::DeliveryFinished {
            claim: first.key(),
            now: 1,
            outcome: DeliveryOutcome::Accepted
        }),
        Err(Error::Conflict)
    );
    assert_eq!(ledger.records()[0].delivery, Some(DeliveryOutcome::Unknown));
    ledger
        .apply(WorkUpdate::DeliveryFinished {
            claim: second.key(),
            now: 1,
            outcome: DeliveryOutcome::Accepted,
        })
        .unwrap();
    assert_eq!(ledger.records()[0].state, WorkState::Done);
}
#[tokio::test]
async fn differently_typed_functions_share_one_worker() {
    for redb in [false, true] {
        const FLAG: Channel<bool> = Channel::new("flag", 1);
        const BOTH: Action<Notice, ()> = Action::new("both", |state, ()| {
            state.count += 1;
            Ok(vec![MAIL.intent("string".into()), FLAG.intent(true)])
        });
        let p = path("typed-functions", redb);
        let store = db(redb, &p);
        let seen = Arc::new(AtomicUsize::new(0));
        let strings = seen.clone();
        let flags = seen.clone();
        let rt = build(
            Runtime::builder()
                .resource(
                    Notice::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true)
                        .action(BOTH),
                )
                .channel(MAIL, service(), move |delivery: Delivery<String>| {
                    assert_eq!(delivery.payload, "string");
                    strings.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                })
                .channel(FLAG, service(), move |delivery: Delivery<bool>| {
                    assert!(delivery.payload);
                    flags.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                }),
            store.clone(),
        );
        rt.execute(
            &actor(),
            Command::create("one", Notice { count: 0 }).idempotency("seed"),
        )
        .await
        .unwrap();
        rt.execute(
            &actor(),
            Command::action("one", BOTH, ())
                .at_revision(1)
                .idempotency("both"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_work(8).await.unwrap(), 2);
        assert_eq!(seen.load(Ordering::SeqCst), 2);
        let records = store.reaction_records().unwrap();
        assert_ne!(records[0].pending.id, records[1].pending.id);
        assert!(records.iter().all(|r| r.state == WorkState::Done));
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn unresolved_last_attempt_reopens_as_unknown_terminal_work() {
    for redb in [false, true] {
        let p = path("last-attempt", redb);
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let limits = ReactionLimits {
            max_attempts: 1,
            lease_seconds: 1,
            ..Default::default()
        };
        let store = db(redb, &p);
        let rt = build(
            builder(clock.clone())
                .reaction_limits(limits.clone())
                .delivery_timeout(std::time::Duration::from_millis(20))
                .channel(MAIL, service(), |_: Delivery<String>| async {
                    panic!("manually claimed attempt")
                }),
            store.clone(),
        );
        enqueue(&rt).await;
        let WorkResult::Claimed(claim) =
            store.reaction_update(WorkUpdate::Claim { now: 0 }).unwrap()
        else {
            panic!()
        };
        store
            .reaction_update(WorkUpdate::DeliveryStarted {
                claim: claim.key(),
                now: 0,
            })
            .unwrap();
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        clock.0.store(1, Ordering::SeqCst);
        let store = db(redb, &p);
        let rt = build(
            builder(clock)
                .reaction_limits(limits)
                .delivery_timeout(std::time::Duration::from_millis(20))
                .channel(MAIL, service(), |_: Delivery<String>| async {
                    panic!("exhausted delivery must not execute")
                }),
            store.clone(),
        );
        assert_eq!(rt.process_work(8).await.unwrap(), 1);
        let record = &store.reaction_records().unwrap()[0];
        assert_eq!(record.attempts, 1);
        assert_eq!(record.delivery, Some(DeliveryOutcome::Unknown));
        assert_eq!(record.state, WorkState::Stopped(StopReason::Attempts));
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(store);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn native_commit_failure_never_detaches_notification_from_resource() {
    for redb in [false, true] {
        for after in [false, true] {
            let p = path(&format!("atomic-{after}"), redb);
            let fail = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = fail.clone();
            let hook: Arc<dyn Fn(usize) -> Result<()> + Send + Sync> = Arc::new(move |point| {
                if point == if after { usize::MAX } else { 0 } && flag.swap(false, Ordering::SeqCst)
                {
                    Err(Error::Storage)
                } else {
                    Ok(())
                }
            });
            let store: Arc<dyn Storage> = if redb {
                let d = Arc::new(rom_redb::Redb::open(&p).unwrap());
                d.on_commit(Some(hook));
                d
            } else {
                let d = Arc::new(rom_sqlite::Sqlite::open(&p).unwrap());
                d.on_commit(Some(hook));
                d
            };
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let rt = build(
                builder(Arc::new(TestClock(AtomicU64::new(0)))).channel(
                    MAIL,
                    service(),
                    move |_: Delivery<String>| {
                        count.fetch_add(1, Ordering::SeqCst);
                        async { DeliveryOutcome::Accepted }
                    },
                ),
                store.clone(),
            );
            rt.execute(
                &actor(),
                Command::create("one", Notice { count: 0 }).idempotency("seed"),
            )
            .await
            .unwrap();
            fail.store(true, Ordering::SeqCst);
            assert!(
                rt.execute(
                    &actor(),
                    Command::action("one", NOTIFY, ())
                        .at_revision(1)
                        .idempotency("notify")
                )
                .await
                .is_err()
            );
            assert_eq!(
                rt.read::<Notice>(&actor(), "one")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .count,
                u64::from(after)
            );
            assert_eq!(store.reaction_records().unwrap().len(), usize::from(after));
            rt.process_work(8).await.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), usize::from(after));
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[test]
fn notification_format_rejects_older_marker_without_migrating() {
    let p = path("old-sqlite", false);
    {
        let connection = rusqlite::Connection::open(&p).unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
    }
    assert!(matches!(
        rom_sqlite::Sqlite::open(&p),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        rusqlite::Connection::open(&p)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
    let _ = std::fs::remove_file(p);
}
#[test]
fn redb_notification_format_rejects_older_marker_without_migrating() {
    use redb::{Database, ReadableDatabase, TableDefinition};
    let p = path("old-redb", true);
    const META: TableDefinition<&str, u64> = TableDefinition::new("rom_metadata");
    {
        let db = Database::create(&p).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(META).unwrap().insert("format", 2).unwrap();
        tx.commit().unwrap();
    }
    assert!(matches!(
        rom_redb::Redb::open(&p),
        Err(Error::Unsupported(_))
    ));
    {
        let db = Database::open(&p).unwrap();
        let tx = db.begin_read().unwrap();
        assert_eq!(
            tx.open_table(META)
                .unwrap()
                .get("format")
                .unwrap()
                .unwrap()
                .value(),
            2
        );
    }
    let _ = std::fs::remove_file(p);
}
#[tokio::test]
async fn channel_requires_current_and_historical_source_field_permission() {
    for redb in [false, true] {
        for hidden in [1, 2] {
            let p = path(&format!("source-fields-{hidden}"), redb);
            let store = db(redb, &p);
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            fn historical(a: &Actor, access: Access, _field: &str, value: &Notice) -> bool {
                a.principal_kind() != PrincipalKind::Service
                    || !matches!(access, Access::Read)
                    || value.count != 1
            }
            fn current(a: &Actor, access: Access, _field: &str, value: &Notice) -> bool {
                a.principal_kind() != PrincipalKind::Service
                    || !matches!(access, Access::Read)
                    || value.count != 2
            }
            let policy = if hidden == 1 { historical } else { current };
            let definition = Notice::definition()
                .policy(|_, _, _| true)
                .field_policy(policy)
                .action(NOTIFY);
            let rt = build(
                Runtime::builder().resource(definition).channel(
                    MAIL,
                    service(),
                    move |_: Delivery<String>| {
                        count.fetch_add(1, Ordering::SeqCst);
                        async { DeliveryOutcome::Accepted }
                    },
                ),
                store.clone(),
            );
            enqueue(&rt).await;
            rt.execute(
                &actor(),
                Command::replace("one", Notice { count: 2 })
                    .at_revision(2)
                    .idempotency("visibility"),
            )
            .await
            .unwrap();
            rt.process_work(8).await.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(
                store.reaction_records().unwrap()[0].state,
                WorkState::Stopped(StopReason::Denied)
            );
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(store);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[tokio::test]
async fn drained_work_releases_its_database_handle_before_reopen() {
    let p = path("drain-reopen", true);
    for iteration in 0..32 {
        let store = db(true, &p);
        let rt = build(
            builder(Arc::new(TestClock(AtomicU64::new(0)))),
            store.clone(),
        );
        if iteration == 0 {
            rt.execute(
                &actor(),
                Command::create("one", Notice { count: 0 }).idempotency("seed"),
            )
            .await
            .unwrap();
        }
        rt.read::<Notice>(&actor(), "one").await.unwrap();
        rt.shutdown().await.unwrap();
        drop(rt);
        assert_eq!(
            Arc::strong_count(&store),
            1,
            "drained task retained the adapter"
        );
        drop(store);
    }
    let _ = std::fs::remove_file(p);
}
#[tokio::test]
async fn status_distinguishes_ready_draining_and_fully_stopped() {
    let p = path("status", false);
    let store = db(false, &p);
    let started = Arc::new(tokio::sync::Notify::new());
    let notify = started.clone();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let wait = Arc::new(std::sync::Mutex::new(Some(wait)));
    let rt = build(
        builder(Arc::new(TestClock(AtomicU64::new(0)))).channel(
            MAIL,
            service(),
            move |_: Delivery<String>| {
                let wait = wait.lock().unwrap().take().unwrap();
                let notify = notify.clone();
                async move {
                    notify.notify_one();
                    wait.await.unwrap();
                    DeliveryOutcome::Accepted
                }
            },
        ),
        store.clone(),
    );
    let initial = rt.status().unwrap();
    assert_eq!(initial.intake, IntakeState::Open);
    assert!(initial.is_ready());
    assert_eq!(initial.registered_resources, 1);
    assert_eq!(initial.registered_channels, 1);
    assert_eq!(initial.registered_reactions, 0);
    assert_eq!(initial.owned_work, 0);
    enqueue(&rt).await;
    let background = rt.clone();
    let work = tokio::spawn(async move { background.process_work(1).await });
    started.notified().await;
    let during = rt.status().unwrap();
    assert_eq!(during.owned_work, 1);
    assert_eq!(during.available_action_permits, 7);
    assert_eq!(during.available_io_permits, 7);
    let background = rt.clone();
    let shutdown = tokio::spawn(async move { background.shutdown().await });
    tokio::task::yield_now().await;
    let draining = rt.status().unwrap();
    assert_eq!(draining.intake, IntakeState::Draining);
    assert!(!draining.is_ready());
    release.send(()).unwrap();
    work.await.unwrap().unwrap();
    shutdown.await.unwrap().unwrap();
    let stopped = rt.status().unwrap();
    assert_eq!(stopped.intake, IntakeState::Stopped);
    assert_eq!(stopped.owned_work, 0);
    assert_eq!(stopped.available_action_permits, 8);
    assert_eq!(stopped.available_io_permits, 8);
    assert_eq!(stopped.available_subscription_permits, 64);
    assert!(!stopped.failed);
    drop(rt);
    assert_eq!(Arc::strong_count(&store), 1);
    drop(store);
    let _ = std::fs::remove_file(p);
}
#[tokio::test]
async fn status_reports_terminal_failure_without_error_payload() {
    struct PanicGate;
    impl ActorGate for PanicGate {
        fn check(&self, _: &Actor, _: &mut dyn AuthorizationRead) -> Result<()> {
            panic!("fixture policy failure")
        }
    }
    let p = path("status-failed", false);
    let store = db(false, &p);
    let rt = build(
        builder(Arc::new(TestClock(AtomicU64::new(0)))).actor_gate(Arc::new(PanicGate)),
        store.clone(),
    );
    assert!(matches!(
        rt.execute(
            &actor(),
            Command::create("one", Notice { count: 0 }).idempotency("seed")
        )
        .await,
        Err(Error::Panicked)
    ));
    let status = rt.status().unwrap();
    assert_eq!(status.intake, IntakeState::Stopped);
    assert!(status.failed);
    assert!(!status.is_ready());
    assert_eq!(status.owned_work, 0);
    assert_eq!(rt.shutdown().await, Err(Error::Panicked));
    drop(rt);
    assert_eq!(Arc::strong_count(&store), 1);
    drop(store);
    let _ = std::fs::remove_file(p);
}
