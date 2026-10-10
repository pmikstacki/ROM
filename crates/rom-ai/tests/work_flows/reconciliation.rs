//! Metadata knowledge is not output publication or permission to regenerate.
use super::*;
use rom_ai::flow::{AiBudget, AiRun, ReservationStatus};

struct MetadataAdapter {
    inner: ObservedAdapter,
    invalid: u8,
    lookups: AtomicU64,
}
impl Provider for MetadataAdapter {
    fn catalog<'a>(&'a self, deadline: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        self.inner.catalog(deadline)
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        self.inner.complete(attempt)
    }
    fn complete_observed<'a>(
        &'a self,
        attempt: &'a PreparedAttempt,
    ) -> AiFuture<'a, rom_ai::DispatchOutcome> {
        self.inner.complete_observed(attempt)
    }
    fn reconcile<'a>(&'a self, evidence: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        self.inner.reconcile(evidence)
    }
    fn reconcile_observed<'a>(
        &'a self,
        prepared: &'a PreparedAttempt,
        evidence: &'a AttemptEvidence,
    ) -> AiFuture<'a, rom_ai::ReconciliationObservation> {
        Box::pin(async move {
            self.lookups.fetch_add(1, Ordering::SeqCst);
            Ok(rom_ai::ReconciliationObservation::Uncertain {
                evidence: AttemptEvidence::new(
                    if self.invalid == 1 {
                        "foreign-attempt"
                    } else {
                        prepared.identity()
                    },
                    None,
                    Some(
                        if self.invalid == 2 {
                            "gen-foreign"
                        } else {
                            evidence.generation_id().unwrap()
                        }
                        .into(),
                    ),
                )?,
                usage: Usage {
                    input_tokens: Some(if self.invalid == 4 { 5 } else { 4 }),
                    cost: Some(rom_ai::UsdNanos(if self.invalid == 3 { 11 } else { 3 })),
                    ..Usage::default()
                },
            })
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_known_cost_sqlite_stays_held_and_replays_after_restart() {
    journey(false, 0, None).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_known_cost_redb_stays_held_and_replays_after_restart() {
    journey(true, 0, None).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn foreign_or_conflicting_metadata_cannot_change_known_usage_or_release_money() {
    for redb in [false, true] {
        for invalid in 1..=4 {
            journey(redb, invalid, None).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_settlement_noncommit_sqlite_repairs_on_pending_replay() {
    journey(false, 0, Some(false)).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_settlement_noncommit_redb_repairs_on_pending_replay() {
    journey(true, 0, Some(false)).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_settlement_lost_ack_sqlite_recovers_receipt_on_pending_replay() {
    journey(false, 0, Some(true)).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_settlement_lost_ack_redb_recovers_receipt_on_pending_replay() {
    journey(true, 0, Some(true)).await;
}

async fn journey(redb: bool, invalid: u8, interruption: Option<bool>) {
    let mut db = Db::new(redb);
    let fault = interruption.map(|after_commit| {
        Arc::new(fault::SettlementFault {
            inner: db.storage.clone(),
            armed: std::sync::atomic::AtomicBool::new(false),
            after_commit,
        })
    });
    if let Some(fault) = &fault {
        db.storage = fault.clone();
    }
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(MetadataAdapter {
        inner: ObservedAdapter {
            inner: Adapter::new(5),
            invalid: 0,
            reconciled: Mutex::new(Vec::new()),
        },
        invalid,
        lookups: AtomicU64::new(0),
    });
    let build = |db: &Db| {
        FlowHost::new(
            service(),
            provider.clone(),
            Arc::new(Authority),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock.clone()))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
    };
    let (runtime, client) = build(&db);
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "metadata-window",
                AiBudget::new(&service(), "metadata-window", rom_ai::UsdNanos(20)).unwrap(),
            )
            .idempotency("grant-metadata-window"),
        )
        .await
        .unwrap();
    let mut input = submission("metadata-held");
    input.policy = RoutingPolicy::new(
        1,
        Vec::new(),
        vec!["paid".into()],
        Some(ModelPrice::new(
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(10),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("metadata-window")
    .unwrap();
    let run = client.submit(&owner(), input).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let held = client.view(&owner(), &run).await.unwrap();
    assert_eq!(held.state(), &RunState::AwaitingReconciliation);
    if let Some(fault) = &fault {
        fault.armed.store(true, Ordering::SeqCst);
    }
    let result = client
        .resume(&owner(), &run, held.revision(), "metadata-lookup")
        .await;
    if invalid == 0 && interruption.is_none() {
        assert!(result.is_ok());
    } else {
        assert!(result.is_err());
    }
    assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
    assert_eq!(provider.inner.inner.calls.load(Ordering::SeqCst), 1);
    let record = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(record.state(), &RunState::AwaitingReconciliation);
    assert_eq!(
        record.attempt_evidence()[0].generation_id(),
        Some("gen-original-header")
    );
    assert_eq!(record.attempt_usage()[0].input_tokens, Some(4));
    assert_eq!(
        record.attempt_usage()[0].cost,
        (invalid == 0).then_some(rom_ai::UsdNanos(3))
    );
    let account = runtime
        .read::<AiBudget>(&service(), "metadata-window")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(
        account.reserved(),
        rom_ai::UsdNanos(if invalid == 0 && interruption != Some(false) {
            0
        } else {
            10
        })
    );
    assert_eq!(
        account.settled(),
        rom_ai::UsdNanos(if invalid == 0 && interruption != Some(false) {
            3
        } else {
            0
        })
    );
    assert_eq!(
        account.entries()[0].status(),
        &if invalid == 0 && interruption != Some(false) {
            ReservationStatus::Settled {
                actual_cost: rom_ai::UsdNanos(3),
            }
        } else {
            ReservationStatus::Unknown
        }
    );
    assert!(
        client
            .view(&owner(), &run)
            .await
            .unwrap()
            .output()
            .is_none()
    );
    if invalid == 0 {
        let original_run_revision = client.view(&owner(), &run).await.unwrap().revision();
        let revision = runtime
            .read::<AiBudget>(&service(), "metadata-window")
            .await
            .unwrap()
            .revision;
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        drop(fault);
        let db = db.reopen();
        let (runtime, client) = build(&db);
        if interruption.is_none() {
            runtime.process_work(8).await.unwrap();
        }
        let replay = client
            .resume(&owner(), &run, held.revision(), "metadata-lookup")
            .await
            .unwrap();
        assert_eq!(replay.state(), &RunState::AwaitingReconciliation);
        assert!(replay.output().is_none());
        assert_eq!(
            replay.revision(),
            original_run_revision + u64::from(interruption.is_some())
        );
        assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
        assert_eq!(provider.inner.inner.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            runtime
                .read::<AiBudget>(&service(), "metadata-window")
                .await
                .unwrap()
                .revision,
            revision + u64::from(interruption == Some(false))
        );
        assert_eq!(
            runtime
                .read::<AiBudget>(&service(), "metadata-window")
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap()
                .settled(),
            rom_ai::UsdNanos(3)
        );
        let again = client
            .resume(&owner(), &run, held.revision(), "metadata-lookup")
            .await
            .unwrap();
        assert_eq!(again.revision(), replay.revision());
        assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    } else {
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_pending_settlement_sqlite_keeps_original_selection_when_successor_advances() {
    selection_race(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metadata_pending_settlement_redb_keeps_original_selection_when_successor_advances() {
    selection_race(true).await;
}
async fn selection_race(redb: bool) {
    let mut db = Db::new(redb);
    let gate = Arc::new(SelectionAuthority {
        phase: std::sync::atomic::AtomicU8::new(0),
        entered: tokio::sync::Semaphore::new(0),
        release: (Mutex::new(false), std::sync::Condvar::new()),
    });
    let fault = Arc::new(fault::SettlementFault {
        inner: db.storage.clone(),
        armed: std::sync::atomic::AtomicBool::new(false),
        after_commit: false,
    });
    db.storage = fault.clone();
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(MetadataAdapter {
        inner: ObservedAdapter {
            inner: Adapter::new(5),
            invalid: 0,
            reconciled: Mutex::new(Vec::new()),
        },
        invalid: 0,
        lookups: AtomicU64::new(0),
    });
    let (runtime, client) = FlowHost::new(
        service(),
        provider.clone(),
        gate.clone(),
        Arc::new(Validator),
        clock.clone(),
    )
    .unwrap()
    .install(Runtime::builder().clock(clock.clone()))
    .unwrap()
    .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
    .unwrap();
    let client = Arc::new(client);
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "selection-window",
                AiBudget::new(&service(), "selection-window", rom_ai::UsdNanos(20)).unwrap(),
            )
            .idempotency("selection-account"),
        )
        .await
        .unwrap();
    let mut input = submission("selection-race");
    input.policy = RoutingPolicy::new(
        1,
        Vec::new(),
        vec!["paid".into()],
        Some(ModelPrice::new(
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(10),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("selection-window")
    .unwrap();
    let run = client.submit(&owner(), input).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let held = client.view(&owner(), &run).await.unwrap();
    fault.armed.store(true, Ordering::SeqCst);
    assert!(
        client
            .resume(&owner(), &run, held.revision(), "selection-lookup")
            .await
            .is_err()
    );
    let original = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    let original_key = original.active_attempt().unwrap().key().clone();
    gate.phase.store(1, Ordering::SeqCst);
    let replay_client = client.clone();
    let replay_run = run.clone();
    let expected = held.revision();
    let replay = tokio::spawn(async move {
        replay_client
            .resume(&owner(), &replay_run, expected, "selection-lookup")
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(1), gate.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    let snapshot = runtime.read::<AiRun>(&service(), &run.0).await.unwrap();
    let current = snapshot.value.unwrap().record().unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::action(
                &run.0,
                rom_ai::flow::CHECKPOINT_RUN,
                rom_ai::flow::CheckpointRun::reconciled_not_accepted(
                    current.checkpoint(),
                    1000,
                    current.attempt_evidence()[0].clone(),
                )
                .unwrap(),
            )
            .at_revision(snapshot.revision)
            .idempotency("selection-confirmed-not-accepted"),
        )
        .await
        .unwrap();
    clock.0.store(2000, Ordering::SeqCst);
    let key = rom_ai::flow::ReservationKey::new("selection-window", &run.0, 2, 2, 1).unwrap();
    let prepared = PreparedAttempt::new(
        key.attempt_identity(),
        original.request().clone(),
        original.policy().clone(),
        original
            .active_attempt()
            .unwrap()
            .prepared()
            .route()
            .clone(),
        Deadline::remaining(2000, 1000, original.expires_at_unix_ms()).unwrap(),
    )
    .unwrap();
    let reservation = rom_ai::flow::ReserveBudget::new(key, prepared).unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::action(
                "selection-window",
                rom_ai::flow::RESERVE_BUDGET,
                reservation.clone(),
            )
            .at_revision(
                runtime
                    .read::<AiBudget>(&service(), "selection-window")
                    .await
                    .unwrap()
                    .revision,
            )
            .idempotency("selection-successor-reserve"),
        )
        .await
        .unwrap();
    let latest = runtime.read::<AiRun>(&service(), &run.0).await.unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::action(
                &run.0,
                rom_ai::flow::PREPARE_RUN,
                rom_ai::flow::PrepareRun::new(reservation.entry().clone(), 2000).unwrap(),
            )
            .at_revision(latest.revision)
            .idempotency("selection-successor-prepare"),
        )
        .await
        .unwrap();
    let successor = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_ne!(successor.active_attempt().unwrap().key(), &original_key);
    let successor_key = successor.active_attempt().unwrap().key().clone();
    *gate.release.0.lock().unwrap() = true;
    gate.release.1.notify_one();
    replay.await.unwrap().unwrap();
    let account = runtime
        .read::<AiBudget>(&service(), "selection-window")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(
        account
            .entries()
            .iter()
            .find(|entry| entry.key() == &original_key)
            .unwrap()
            .status(),
        &ReservationStatus::Settled {
            actual_cost: rom_ai::UsdNanos(3)
        }
    );
    assert_eq!(
        account
            .entries()
            .iter()
            .find(|entry| entry.key() == &successor_key)
            .unwrap()
            .status(),
        &ReservationStatus::Reserved
    );
    assert_eq!(account.reserved(), rom_ai::UsdNanos(10));
    assert_eq!(account.settled(), rom_ai::UsdNanos(3));
    assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
    assert_eq!(provider.inner.inner.calls.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
}

struct SelectionAuthority {
    phase: std::sync::atomic::AtomicU8,
    entered: tokio::sync::Semaphore,
    release: (Mutex<bool>, std::sync::Condvar),
}
impl FlowAuthority for SelectionAuthority {
    fn submit(&self, actor: &Actor, input: &Submission) -> AiResult<OwnerIdentity> {
        Authority.submit(actor, input)
    }
    fn cancel(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        Authority.cancel(actor, identity)
    }
    fn resolve(
        &self,
        identity: &OwnerIdentity,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        Authority.resolve(identity, reads)
    }
    fn attempt(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        prepared: &PreparedAttempt,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        Authority.attempt(actor, identity, prepared, reads)?;
        let _ = self
            .phase
            .compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst);
        Ok(())
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        Authority.inspect(actor, identity)?;
        if self
            .phase
            .compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.entered.add_permits(1);
            let (released, _) = self
                .release
                .1
                .wait_timeout_while(
                    self.release.0.lock().unwrap(),
                    std::time::Duration::from_secs(5),
                    |released| !*released,
                )
                .unwrap();
            if !*released {
                return Err(rom_ai::AiError::Denied);
            }
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_held_knowledge_advances_only_latest_milestone_timestamp() {
    advancing_knowledge(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_held_knowledge_advances_only_latest_milestone_timestamp() {
    advancing_knowledge(true).await;
}
async fn advancing_knowledge(redb: bool) {
    use rom_ai::flow::{
        CHECKPOINT_RUN, CheckpointRun, RECORD_ATTEMPT_EVIDENCE, RecordAttemptEvidence,
        SETTLE_BUDGET, SettleBudget,
    };
    let db = Db::new(redb);
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(ObservedAdapter {
        inner: Adapter::new(5),
        invalid: 0,
        reconciled: Mutex::new(Vec::new()),
    });
    let (runtime, client) = FlowHost::new(
        service(),
        provider.clone(),
        Arc::new(Authority),
        Arc::new(Validator),
        clock.clone(),
    )
    .unwrap()
    .install(Runtime::builder().clock(clock.clone()))
    .unwrap()
    .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
    .unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "advancing-window",
                AiBudget::new(&service(), "advancing-window", rom_ai::UsdNanos(20)).unwrap(),
            )
            .idempotency("advancing-account"),
        )
        .await
        .unwrap();
    let mut input = submission("advancing-run");
    input.policy = RoutingPolicy::new(
        1,
        Vec::new(),
        vec!["paid".into()],
        Some(ModelPrice::new(
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(10),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("advancing-window")
    .unwrap();
    let run = client.submit(&owner(), input).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let snapshot = runtime.read::<AiRun>(&service(), &run.0).await.unwrap();
    let first = snapshot.value.unwrap().record().unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::action(
                &run.0,
                CHECKPOINT_RUN,
                CheckpointRun::reconciled_not_accepted(
                    first.checkpoint(),
                    1000,
                    first.attempt_evidence()[0].clone(),
                )
                .unwrap(),
            )
            .at_revision(snapshot.revision)
            .idempotency("advancing-original-nonacceptance"),
        )
        .await
        .unwrap();
    let budget = runtime
        .read::<AiBudget>(&service(), "advancing-window")
        .await
        .unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::action(
                "advancing-window",
                SETTLE_BUDGET,
                SettleBudget::confirmed(
                    first.active_attempt().unwrap().key().clone(),
                    rom_ai::UsdNanos(0),
                )
                .unwrap(),
            )
            .at_revision(budget.revision)
            .idempotency("advancing-original-settle"),
        )
        .await
        .unwrap();
    clock.0.store(2000, Ordering::SeqCst);
    runtime.process_work(8).await.unwrap();
    let before_view = client.view(&owner(), &run).await.unwrap();
    assert_eq!(before_view.milestones().len(), 2);
    let snapshot = runtime.read::<AiRun>(&service(), &run.0).await.unwrap();
    let before = snapshot.value.unwrap().record().unwrap();
    let before_account = runtime
        .read::<AiBudget>(&service(), "advancing-window")
        .await
        .unwrap();
    let mut usage = before.attempt_usage()[1].clone();
    usage.cost = Some(rom_ai::UsdNanos(3));
    clock.0.store(3000, Ordering::SeqCst);
    runtime
        .execute(
            &service(),
            rom::Command::action(
                &run.0,
                RECORD_ATTEMPT_EVIDENCE,
                RecordAttemptEvidence::new(
                    before.checkpoint(),
                    3000,
                    before.attempt_evidence()[1].clone(),
                    usage,
                )
                .unwrap(),
            )
            .at_revision(snapshot.revision)
            .idempotency("advancing-known-cost"),
        )
        .await
        .expect("advancing held knowledge must satisfy the milestone timestamp invariant");
    let after = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    let after_view = client.view(&owner(), &run).await.unwrap();
    assert_eq!(after.last_observed_unix_ms(), 3000);
    assert_eq!(after_view.milestones()[1].observed_at_unix_ms(), 3000);
    assert_eq!(after_view.milestones()[0], before_view.milestones()[0]);
    assert_eq!(
        after_view.milestones().len(),
        before_view.milestones().len()
    );
    assert_eq!(
        after_view.milestones()[1].sequence(),
        before_view.milestones()[1].sequence()
    );
    assert_eq!(
        after_view.milestones()[1].state(),
        before_view.milestones()[1].state()
    );
    assert_eq!(after.counters(), before.counters());
    assert_eq!(after.active_attempt(), before.active_attempt());
    assert_eq!(after.owner(), before.owner());
    assert_eq!(after.request(), before.request());
    assert_eq!(after.policy(), before.policy());
    assert_eq!(after.cursor(), before.cursor());
    assert_eq!(after.state(), before.state());
    assert!(after_view.output().is_none());
    assert_eq!(after.attempt_usage()[1].cost, Some(rom_ai::UsdNanos(3)));
    let after_account = runtime
        .read::<AiBudget>(&service(), "advancing-window")
        .await
        .unwrap();
    assert_eq!(after_account.revision, before_account.revision);
    assert_eq!(
        after_account.value.unwrap().record().unwrap(),
        before_account.value.unwrap().record().unwrap()
    );
    assert_eq!(provider.inner.calls.load(Ordering::SeqCst), 2);
    runtime.shutdown().await.unwrap();
}
