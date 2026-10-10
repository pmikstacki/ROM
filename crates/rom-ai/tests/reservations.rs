mod support;
use rom::{Actor, Command, PrincipalKind, Runtime};
use rom_ai::flow::{
    AiBudget, AiRun, HOLD_RUN, HoldRun, OwnerIdentity, PREPARE_RUN, PrepareRun, RESERVE_BUDGET,
    ReservationKey, ReservationStatus, ReserveBudget, RunState, SETTLE_BUDGET, START_RUN,
    SettleBudget, StartRun,
};
use rom_ai::{
    CatalogModel, CatalogSnapshot, CompletionRequest, Deadline, Message, ModelPrice,
    PreparedAttempt, RouteCursor, RoutingPolicy, RunLimits, UsdNanos, choose,
};
use support::Fixture;

fn service() -> Actor {
    Actor::trusted("tests", "ai").with_kind(PrincipalKind::Service)
}
fn request(text: &str) -> CompletionRequest {
    CompletionRequest::new(vec![Message::user(text)], 256).unwrap()
}
fn policy() -> RoutingPolicy {
    RoutingPolicy::new(
        1,
        vec![],
        vec!["paid".into()],
        Some(ModelPrice::new(
            UsdNanos(100_000_000),
            UsdNanos(400_000_000),
            UsdNanos(0),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("window-1")
    .unwrap()
}
fn reservation(run: &str, text: &str) -> ReserveBudget {
    reservation_at(run, text, 1, 1, 1000)
}
fn reservation_at(run: &str, text: &str, step: u32, ordinal: u32, now: u64) -> ReserveBudget {
    let request = request(text);
    let policy = policy();
    let catalog = CatalogSnapshot::new(
        "fixture",
        vec![CatalogModel::text(
            "paid",
            262144,
            true,
            true,
            policy.paid_caps().unwrap(),
        )],
    )
    .unwrap();
    let route = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "fixture").unwrap(),
        &request,
    )
    .unwrap();
    let key = ReservationKey::new("window-1", run, step, ordinal, 1).unwrap();
    let prepared = PreparedAttempt::new(
        key.attempt_identity(),
        request,
        policy,
        route,
        Deadline::remaining(now, now, now + 1000).unwrap(),
    )
    .unwrap();
    ReserveBudget::new(key, prepared).unwrap()
}

async fn executing(f: &Fixture) -> ReserveBudget {
    let input = reservation("run-1", "public task");
    seed(f, "run-1", UsdNanos(1000000)).await;
    f.runtime()
        .execute(
            &service(),
            Command::action("window-1", RESERVE_BUDGET, input.clone())
                .at_revision(1)
                .idempotency("reserve"),
        )
        .await
        .unwrap();
    f.runtime()
        .execute(
            &service(),
            Command::action(
                "run-1",
                PREPARE_RUN,
                PrepareRun::new(input.entry().clone(), 1000).unwrap(),
            )
            .at_revision(1)
            .idempotency("prepare"),
        )
        .await
        .unwrap();
    f.runtime()
        .execute(
            &service(),
            Command::action("run-1", START_RUN, StartRun::new(1, 1001).unwrap())
                .at_revision(2)
                .idempotency("start"),
        )
        .await
        .unwrap();
    input
}
fn evidence(input: &ReserveBudget) -> rom_ai::AttemptEvidence {
    rom_ai::AttemptEvidence::new(input.entry().prepared().identity(), None, None).unwrap()
}

#[tokio::test]
async fn reconciled_nonacceptance_is_explicit_atomic_and_preserves_evidence() {
    use rom_ai::flow::{CHECKPOINT_RUN, CheckpointRun};
    for redb in [false, true] {
        let mut f = Fixture::new(redb);
        let input = executing(&f).await;
        let known = rom_ai::AttemptEvidence::new(
            input.entry().prepared().identity(),
            Some("provider-request".into()),
            Some("generation".into()),
        )
        .unwrap();
        let wrong = rom_ai::AttemptEvidence::new("wrong-attempt", None, None).unwrap();
        let before = f.counts();
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        HOLD_RUN,
                        HoldRun::new(1, 1002)
                            .unwrap()
                            .with_evidence(wrong.clone())
                            .unwrap()
                    )
                    .at_revision(3)
                    .idempotency("wrong-hold")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    HOLD_RUN,
                    HoldRun::new(1, 1002)
                        .unwrap()
                        .with_evidence(known.clone())
                        .unwrap(),
                )
                .at_revision(3)
                .idempotency("hold-known"),
            )
            .await
            .unwrap();
        f.reopen().await;
        let before = f.counts();
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        CHECKPOINT_RUN,
                        CheckpointRun::waiting(1, 1003, 2000, known.clone()).unwrap()
                    )
                    .at_revision(4)
                    .idempotency("blind-wait")
                )
                .await
                .is_err()
        );
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        CHECKPOINT_RUN,
                        CheckpointRun::reconciled_not_accepted(1, 1003, wrong).unwrap()
                    )
                    .at_revision(4)
                    .idempotency("wrong-reconcile")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        let command = Command::action(
            "run-1",
            CHECKPOINT_RUN,
            CheckpointRun::reconciled_not_accepted(1, 1003, known.clone()).unwrap(),
        )
        .at_revision(4)
        .idempotency("confirmed-not-accepted");
        f.runtime()
            .execute(&service(), command.clone())
            .await
            .unwrap();
        let after = f.counts();
        f.runtime().execute(&service(), command).await.unwrap();
        assert_eq!(f.counts(), after);
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            run.state(),
            &RunState::Waiting {
                retry_at_unix_ms: 2000
            }
        );
        assert_eq!(run.attempt_evidence(), &[known]);
        assert_eq!(run.counters().generation_attempts(), 1);
        assert_eq!(f.work().len(), 1);
        assert_eq!(f.work()[0].pending.not_before, Some(2));
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            budget.reserved(),
            input.entry().prepared().route().maximum_cost()
        );
    }
}

#[tokio::test]
async fn expired_reconciliation_retains_charge_without_wake() {
    use rom_ai::flow::{CHECKPOINT_RUN, CheckpointRun};
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let input = executing(&f).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", HOLD_RUN, HoldRun::new(1, 1002).unwrap())
                    .at_revision(3)
                    .idempotency("hold"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    CHECKPOINT_RUN,
                    CheckpointRun::reconciled_not_accepted(1, 302000, evidence(&input)).unwrap(),
                )
                .at_revision(4)
                .idempotency("expired-reconcile"),
            )
            .await
            .unwrap();
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::Failed);
        assert_eq!(run.failure(), Some(&rom_ai::AiError::DeadlineExceeded));
        assert!(f.work().is_empty());
        assert_eq!(run.counters().generation_attempts(), 1);
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            budget.reserved(),
            input.entry().prepared().route().maximum_cost()
        );
    }
}

#[tokio::test]
async fn waiting_checkpoint_is_atomic_and_replay_preserves_rounded_due() {
    use rom_ai::flow::{CHECKPOINT_RUN, CheckpointRun};
    for redb in [false, true] {
        let mut f = Fixture::new(redb);
        let first = executing(&f).await;
        let command = Command::action(
            "run-1",
            CHECKPOINT_RUN,
            CheckpointRun::waiting(1, 1002, 2001, evidence(&first)).unwrap(),
        )
        .at_revision(3)
        .idempotency("wait");
        f.runtime()
            .execute(&service(), command.clone())
            .await
            .unwrap();
        let before = f.counts();
        f.runtime().execute(&service(), command).await.unwrap();
        assert_eq!(f.counts(), before);
        f.reopen().await;
        let work = f.work();
        assert_eq!(work.len(), 1);
        assert_eq!(work[0].pending.not_before, Some(3));
        assert_eq!(work[0].due, 3);
        assert_eq!(work[0].attempts, 0);
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            run.state(),
            &RunState::Waiting {
                retry_at_unix_ms: 3000
            }
        );
        assert_eq!(run.attempt_evidence().len(), 1);
        let early = reservation_at("run-1", "public task", 2, 2, 2999);
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        PREPARE_RUN,
                        PrepareRun::new(early.entry().clone(), 2999).unwrap()
                    )
                    .at_revision(4)
                    .idempotency("early")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        let next = reservation_at("run-1", "public task", 2, 2, 3000);
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, next.clone())
                    .at_revision(2)
                    .idempotency("next-reserve"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    PREPARE_RUN,
                    PrepareRun::new(next.entry().clone(), 3000).unwrap(),
                )
                .at_revision(4)
                .idempotency("next-prepare"),
            )
            .await
            .unwrap();
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.counters().generation_attempts(), 2);
        assert_eq!(run.counters().ticks(), 2);
        assert_eq!(run.attempt_evidence().len(), 1);
    }
}

#[tokio::test]
async fn completed_checkpoint_checks_attempt_and_authorized_projection() {
    use rom_ai::flow::{CHECKPOINT_RUN, CheckpointRun, RunView};
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let input = executing(&f).await;
        let before = f.counts();
        let wrong = rom_ai::AttemptEvidence::new("external-attempt", None, None).unwrap();
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        CHECKPOINT_RUN,
                        CheckpointRun::completed(
                            1,
                            1002,
                            wrong,
                            rom_ai::Usage::default(),
                            serde_json::json!({"answer":"private validated result"})
                        )
                        .unwrap()
                    )
                    .at_revision(3)
                    .idempotency("wrong-evidence")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    CHECKPOINT_RUN,
                    CheckpointRun::completed(
                        1,
                        1002,
                        evidence(&input),
                        rom_ai::Usage::default(),
                        serde_json::json!({"answer":"private validated result"}),
                    )
                    .unwrap(),
                )
                .at_revision(3)
                .idempotency("complete"),
            )
            .await
            .unwrap();
        assert!(f.work().is_empty());
        let snapshot = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap();
        let run = snapshot.value.unwrap().record().unwrap();
        let owner = Actor::trusted("tests", "owner");
        assert!(
            RunView::project(&run, snapshot.revision, &owner, |_, _| Err(
                rom_ai::AiError::Denied
            ))
            .is_err()
        );
        let view = RunView::project(&run, snapshot.revision, &owner, |actor, identity| {
            if identity.matches(actor) {
                Ok(())
            } else {
                Err(rom_ai::AiError::Denied)
            }
        })
        .unwrap();
        assert_eq!(view.state(), &RunState::Completed);
        assert_eq!(
            view.output(),
            Some(&serde_json::json!({"answer":"private validated result"}))
        );
        assert!(!format!("{view:?}").contains("private validated result"));
        let wire = serde_json::to_value(&view).unwrap();
        assert!(wire.get("request").is_none());
        assert!(wire.get("service").is_none());
        assert!(wire.get("account_window").is_none());
    }
}

#[tokio::test]
async fn cancellation_retains_unknown_cost_and_hides_late_output() {
    use rom_ai::flow::{CANCEL_RUN, CHECKPOINT_RUN, CancelRun, CheckpointRun, RunView};
    for redb in [false, true] {
        let f = Fixture::new(redb);
        seed(&f, "run-1", UsdNanos(1000000)).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", CANCEL_RUN, CancelRun::new(0, 1001).unwrap())
                    .at_revision(1)
                    .idempotency("queued-cancel"),
            )
            .await
            .unwrap();
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::Cancelled);
        assert_eq!(run.counters().generation_attempts(), 0);
        let before = f.counts();
        let queued_attempt = reservation("run-1", "public task");
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        PREPARE_RUN,
                        PrepareRun::new(queued_attempt.entry().clone(), 1000).unwrap()
                    )
                    .at_revision(2)
                    .idempotency("cancelled-prepare")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        let prepared_fixture = Fixture::new(redb);
        let prepared_attempt = reservation("run-1", "public task");
        seed(&prepared_fixture, "run-1", UsdNanos(1000000)).await;
        prepared_fixture
            .runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, prepared_attempt.clone())
                    .at_revision(1)
                    .idempotency("prepared-reserve"),
            )
            .await
            .unwrap();
        prepared_fixture
            .runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    PREPARE_RUN,
                    PrepareRun::new(prepared_attempt.entry().clone(), 1000).unwrap(),
                )
                .at_revision(1)
                .idempotency("prepared-prepare"),
            )
            .await
            .unwrap();
        prepared_fixture
            .runtime()
            .execute(
                &service(),
                Command::action("run-1", CANCEL_RUN, CancelRun::new(1, 1001).unwrap())
                    .at_revision(2)
                    .idempotency("prepared-cancel"),
            )
            .await
            .unwrap();
        let before = prepared_fixture.counts();
        assert!(
            prepared_fixture
                .runtime()
                .execute(
                    &service(),
                    Command::action("run-1", START_RUN, StartRun::new(1, 1002).unwrap())
                        .at_revision(3)
                        .idempotency("cancelled-start")
                )
                .await
                .is_err()
        );
        assert_eq!(prepared_fixture.counts(), before);
        let f = Fixture::new(redb);
        let input = executing(&f).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", CANCEL_RUN, CancelRun::new(1, 1002).unwrap())
                    .at_revision(3)
                    .idempotency("started-cancel"),
            )
            .await
            .unwrap();
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::CancelRequested);
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", HOLD_RUN, HoldRun::new(1, 1003).unwrap())
                    .at_revision(4)
                    .idempotency("unknown-after-cancel"),
            )
            .await
            .unwrap();
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::AwaitingReconciliation);
        assert!(run.cancel_requested());
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    CHECKPOINT_RUN,
                    CheckpointRun::completed(
                        1,
                        302000,
                        evidence(&input),
                        rom_ai::Usage::default(),
                        serde_json::json!("late private result"),
                    )
                    .unwrap(),
                )
                .at_revision(5)
                .idempotency("late-result"),
            )
            .await
            .unwrap();
        let snapshot = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap();
        let run = snapshot.value.unwrap().record().unwrap();
        assert_eq!(run.state(), &RunState::Cancelled);
        assert_eq!(run.expires_at_unix_ms(), 301000);
        assert_eq!(run.counters().generation_attempts(), 1);
        let view = RunView::project(
            &run,
            snapshot.revision,
            &Actor::trusted("tests", "owner"),
            |_, _| Ok(()),
        )
        .unwrap();
        assert!(view.output().is_none());
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.reserved(), input.entry().maximum_cost());
        assert!(f.work().is_empty());
        let recon_fixture = Fixture::new(redb);
        executing(&recon_fixture).await;
        recon_fixture
            .runtime()
            .execute(
                &service(),
                Command::action("run-1", HOLD_RUN, HoldRun::new(1, 1002).unwrap())
                    .at_revision(3)
                    .idempotency("recon-hold"),
            )
            .await
            .unwrap();
        recon_fixture
            .runtime()
            .execute(
                &service(),
                Command::action("run-1", CANCEL_RUN, CancelRun::new(1, 1003).unwrap())
                    .at_revision(4)
                    .idempotency("recon-cancel"),
            )
            .await
            .unwrap();
        let run = recon_fixture
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::AwaitingReconciliation);
        assert!(run.cancel_requested());
    }
}

#[tokio::test]
async fn rounded_retry_at_expiry_fails_without_successor() {
    use rom_ai::flow::{CHECKPOINT_RUN, CheckpointRun};
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let input = executing(&f).await;
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    CHECKPOINT_RUN,
                    CheckpointRun::waiting(1, 1002, 300999, evidence(&input)).unwrap(),
                )
                .at_revision(3)
                .idempotency("rounded-expiry"),
            )
            .await
            .unwrap();
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::Failed);
        assert_eq!(run.failure(), Some(&rom_ai::AiError::DeadlineExceeded));
        assert!(f.work().is_empty());
    }
}
async fn seed(f: &Fixture, run: &str, limit: UsdNanos) {
    f.runtime()
        .execute(
            &service(),
            Command::create(
                "window-1",
                AiBudget::new(&service(), "window-1", limit).unwrap(),
            )
            .idempotency("budget-create"),
        )
        .await
        .unwrap();
    seed_run(f.runtime(), run).await;
}
async fn seed_run(runtime: &Runtime, run: &str) {
    let owner = OwnerIdentity::from_actor(&Actor::trusted("tests", "owner")).unwrap();
    runtime
        .execute(
            &service(),
            Command::create(
                run,
                AiRun::queued(
                    run,
                    &service(),
                    owner,
                    request("public task"),
                    policy(),
                    1000,
                )
                .unwrap(),
            )
            .idempotency(&format!("create-{run}")),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn concurrent_reservations_authorize_one_dispatch() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let one = reservation("run-1", "public task");
        let two = reservation("run-2", "public task");
        seed(&f, "run-1", one.entry().maximum_cost()).await;
        seed_run(f.runtime(), "run-2").await;
        let command1 = Command::action("window-1", RESERVE_BUDGET, one.clone())
            .at_revision(1)
            .idempotency("reserve-1");
        let command2 = Command::action("window-1", RESERVE_BUDGET, two.clone())
            .at_revision(1)
            .idempotency("reserve-2");
        let actor = service();
        let (a, b) = tokio::join!(
            f.runtime().execute(&actor, command1.clone()),
            f.runtime().execute(&actor, command2.clone())
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        assert!(matches!(
            if a.is_err() { &a } else { &b },
            Err(rom::Error::Conflict)
        ));
        let (winner, replay, loser) = if a.is_ok() {
            (one, command1, two)
        } else {
            (two, command2, one)
        };
        let before = f.counts();
        f.runtime().execute(&actor, replay).await.unwrap();
        assert_eq!(f.counts(), before, "receipt replay wrote again");
        assert!(
            f.runtime()
                .execute(
                    &actor,
                    Command::action("window-1", RESERVE_BUDGET, loser)
                        .at_revision(2)
                        .idempotency("retry-loser")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before, "capacity rejection changed persistence");
        let changed = reservation(winner.entry().key().run_id(), "secret task");
        assert_eq!(
            changed.entry().maximum_cost(),
            winner.entry().maximum_cost()
        );
        assert!(matches!(
            f.runtime()
                .execute(
                    &actor,
                    Command::action("window-1", RESERVE_BUDGET, changed)
                        .at_revision(2)
                        .idempotency("changed-reservation")
                )
                .await,
            Err(rom::Error::IdentityMismatch)
        ));
        assert_eq!(f.counts(), before);
        let budget = f
            .runtime()
            .read::<AiBudget>(&actor, "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.entries().len(), 1);
        assert_eq!(budget.reserved(), winner.entry().maximum_cost());
        f.runtime().revoke(&service());
        let before = f.counts();
        assert!(
            f.runtime()
                .read::<AiBudget>(&service(), "window-1")
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
    }
}

#[tokio::test]
async fn ledger_byte_limit_and_private_codecs_fail_atomically() {
    use rom::Resource;
    for redb in [false, true] {
        let f = Fixture::new(redb);
        f.runtime()
            .execute(
                &service(),
                Command::create(
                    "window-1",
                    AiBudget::new(&service(), "window-1", UsdNanos(u64::MAX)).unwrap(),
                )
                .idempotency("budget-create"),
            )
            .await
            .unwrap();
        let text = "s".repeat(31000);
        let mut revision = 1;
        let mut rejected = false;
        for ordinal in 0..16 {
            let before = f.counts();
            let input = reservation(&format!("large-run-{ordinal}"), &text);
            let result = f
                .runtime()
                .execute(
                    &service(),
                    Command::action("window-1", RESERVE_BUDGET, input)
                        .at_revision(revision)
                        .idempotency(&format!("reserve-{ordinal}")),
                )
                .await;
            if result.is_err() {
                assert_eq!(f.counts(), before);
                rejected = true;
                break;
            }
            revision += 1;
        }
        assert!(rejected, "ledger exceeded its finite byte budget");
        let row = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap();
        let encoded = row.encode();
        let storage = encoded["encoded"].as_str().unwrap();
        assert!(storage.len() <= 256 * 1024);
        let record: serde_json::Value = serde_json::from_str(storage).unwrap();
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("reserved", serde_json::json!(0)),
            ("unexpected", serde_json::json!(true)),
        ] {
            let mut changed = record.clone();
            changed[field] = value;
            let forged = AiBudget::decode(
                serde_json::json!({"encoded":serde_json::to_string(&changed).unwrap()}),
            )
            .unwrap();
            assert!(forged.record().is_err());
            let before = f.counts();
            assert!(
                f.runtime()
                    .execute(
                        &service(),
                        Command::replace("window-1", forged)
                            .at_revision(revision)
                            .idempotency(&format!("forge-{field}"))
                    )
                    .await
                    .is_err()
            );
            assert_eq!(f.counts(), before);
        }
        assert!(!format!("{row:?}").contains(&text));
    }
}

#[tokio::test]
async fn trusted_settlement_is_exact_and_cannot_rewrite_known_usage() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let input = reservation("run-1", "public task");
        seed(&f, "run-1", input.entry().maximum_cost()).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, input.clone())
                    .at_revision(1)
                    .idempotency("reserve"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "window-1",
                    SETTLE_BUDGET,
                    SettleBudget::unknown(input.entry().key().clone()).unwrap(),
                )
                .at_revision(2)
                .idempotency("unknown"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "window-1",
                    SETTLE_BUDGET,
                    SettleBudget::confirmed(input.entry().key().clone(), UsdNanos(11)).unwrap(),
                )
                .at_revision(3)
                .idempotency("settle"),
            )
            .await
            .unwrap();
        let row = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(row.reserved(), UsdNanos(0));
        assert_eq!(row.settled(), UsdNanos(11));
        let before = f.counts();
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "window-1",
                        SETTLE_BUDGET,
                        SettleBudget::confirmed(input.entry().key().clone(), UsdNanos(0)).unwrap()
                    )
                    .at_revision(4)
                    .idempotency("rewrite-known")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, input)
                    .at_revision(4)
                    .idempotency("replay-settled-reservation"),
            )
            .await
            .unwrap();
        let row = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(row.reserved(), UsdNanos(0));
        assert_eq!(row.settled(), UsdNanos(11));
    }
}

#[tokio::test]
async fn decoded_and_patched_policy_cannot_amplify_frozen_authority() {
    use rom::{Patch, Resource};
    for redb in [false, true] {
        let f = Fixture::new(redb);
        seed(&f, "run-1", UsdNanos(1000000)).await;
        let original = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap();
        let mut encoded: serde_json::Value =
            serde_json::from_str(original.encode()["encoded"].as_str().unwrap()).unwrap();
        encoded["policy"]["version"] = serde_json::json!(2);
        encoded["policy"]["paid_caps"]["prompt_per_million"] = serde_json::json!(900000000);
        let encoded = serde_json::to_string(&encoded).unwrap();
        let forged = AiRun::decode(serde_json::json!({"encoded":encoded})).unwrap();
        assert!(
            forged.record().is_ok(),
            "fixture must be a valid new policy, not a malformed codec"
        );
        let before = f.counts();
        assert!(matches!(
            f.runtime()
                .execute(
                    &service(),
                    Command::replace("run-1", forged)
                        .at_revision(1)
                        .idempotency("amplify-replace")
                )
                .await,
            Err(rom::Error::Conflict)
        ));
        assert_eq!(f.counts(), before);
        assert!(matches!(
            f.runtime()
                .execute(
                    &service(),
                    Command::patch("run-1", Patch::new().set(AiRun::encoded_field(), encoded))
                        .at_revision(1)
                        .idempotency("amplify-patch")
                )
                .await,
            Err(rom::Error::Conflict)
        ));
        assert_eq!(f.counts(), before);
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.policy(), &policy());
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.reserved(), UsdNanos(0));
        assert!(budget.entries().is_empty());
    }
}

#[tokio::test]
async fn restart_retains_unknown_cost_and_limits() {
    for redb in [false, true] {
        let mut f = Fixture::new(redb);
        let reserved = reservation("run-1", "public task");
        seed(&f, "run-1", reserved.entry().maximum_cost()).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, reserved.clone())
                    .at_revision(1)
                    .idempotency("reserve"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    PREPARE_RUN,
                    PrepareRun::new(reserved.entry().clone(), 1000).unwrap(),
                )
                .at_revision(1)
                .idempotency("prepare"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", START_RUN, StartRun::new(1, 1001).unwrap())
                    .at_revision(2)
                    .idempotency("start"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", HOLD_RUN, HoldRun::new(1, 1002).unwrap())
                    .at_revision(3)
                    .idempotency("hold"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "window-1",
                    SETTLE_BUDGET,
                    SettleBudget::unknown(reserved.entry().key().clone()).unwrap(),
                )
                .at_revision(2)
                .idempotency("unknown"),
            )
            .await
            .unwrap();
        f.reopen().await;
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::AwaitingReconciliation);
        assert_eq!(run.counters().generation_attempts(), 1);
        assert_eq!(run.counters().ticks(), 1);
        assert_eq!(run.active_attempt().unwrap(), reserved.entry());
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.reserved(), reserved.entry().maximum_cost());
        assert_eq!(budget.entries()[0].status(), &ReservationStatus::Unknown);
        let before = f.counts();
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action(
                        "run-1",
                        PREPARE_RUN,
                        PrepareRun::new(reserved.entry().clone(), 1000).unwrap()
                    )
                    .at_revision(4)
                    .idempotency("blind-retry")
                )
                .await
                .is_err()
        );
        assert_eq!(f.counts(), before);
        assert!(
            f.runtime()
                .read::<AiRun>(&Actor::trusted("tests", "owner"), "run-1")
                .await
                .is_err()
        );
        assert!(
            f.runtime()
                .execute(
                    &Actor::trusted("tests", "other").with_kind(PrincipalKind::Service),
                    Command::action("window-1", RESERVE_BUDGET, reserved)
                        .at_revision(3)
                        .idempotency("forged-service")
                )
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn reservation_then_run_commit_is_conservative() {
    for redb in [false, true] {
        let mut f = Fixture::new(redb);
        let reserved = reservation("run-1", "public task");
        seed(&f, "run-1", reserved.entry().maximum_cost()).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, reserved.clone())
                    .at_revision(1)
                    .idempotency("reserve"),
            )
            .await
            .unwrap();
        // Inject process interruption between two public Resource commits. No provider exists here.
        f.reopen().await;
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.state(), &RunState::Queued);
        assert_eq!(run.counters().generation_attempts(), 0);
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.reserved(), reserved.entry().maximum_cost());
        assert_eq!(budget.entries()[0], *reserved.entry());
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, reserved.clone())
                    .at_revision(2)
                    .idempotency("recover-reservation"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    PREPARE_RUN,
                    PrepareRun::new(reserved.entry().clone(), 1000).unwrap(),
                )
                .at_revision(1)
                .idempotency("prepare"),
            )
            .await
            .unwrap();
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.entries().len(), 1);
        assert_eq!(budget.reserved(), reserved.entry().maximum_cost());
    }
}

#[tokio::test]
async fn free_attempt_needs_no_paid_authority() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let policy =
            RoutingPolicy::new(1, vec!["free".into()], vec![], None, RunLimits::default()).unwrap();
        let request = request("public task");
        let catalog = CatalogSnapshot::new(
            "fixture",
            vec![CatalogModel::text(
                "free",
                8192,
                true,
                true,
                ModelPrice::free(),
            )],
        )
        .unwrap();
        let route = choose(
            &policy,
            &catalog,
            &RouteCursor::new(1, "fixture").unwrap(),
            &request,
        )
        .unwrap();
        let key = ReservationKey::new("free-window", "run-1", 1, 1, 1).unwrap();
        let prepared = PreparedAttempt::new(
            key.attempt_identity(),
            request,
            policy,
            route,
            Deadline::remaining(1000, 1000, 2000).unwrap(),
        )
        .unwrap();
        let reservation =
            ReserveBudget::new(key, prepared).expect("free routing required paid budget authority");
        f.runtime()
            .execute(
                &service(),
                Command::create(
                    "free-window",
                    AiBudget::new(&service(), "free-window", UsdNanos(0)).unwrap(),
                )
                .idempotency("free-budget"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action("free-window", RESERVE_BUDGET, reservation)
                    .at_revision(1)
                    .idempotency("free-reserve"),
            )
            .await
            .unwrap();
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "free-window")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.reserved(), UsdNanos(0));
        assert_eq!(budget.entries().len(), 1);
    }
}

#[tokio::test]
async fn late_unknown_evidence_does_not_extend_or_release_expired_run() {
    for redb in [false, true] {
        let f = Fixture::new(redb);
        let reservation = reservation("run-1", "public task");
        seed(&f, "run-1", reservation.entry().maximum_cost()).await;
        f.runtime()
            .execute(
                &service(),
                Command::action("window-1", RESERVE_BUDGET, reservation.clone())
                    .at_revision(1)
                    .idempotency("reserve"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action(
                    "run-1",
                    PREPARE_RUN,
                    PrepareRun::new(reservation.entry().clone(), 1000).unwrap(),
                )
                .at_revision(1)
                .idempotency("prepare"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", START_RUN, StartRun::new(1, 1001).unwrap())
                    .at_revision(2)
                    .idempotency("start"),
            )
            .await
            .unwrap();
        f.runtime()
            .execute(
                &service(),
                Command::action("run-1", HOLD_RUN, HoldRun::new(1, 302000).unwrap())
                    .at_revision(3)
                    .idempotency("late-hold"),
            )
            .await
            .expect("expired execution lost its reconciliation checkpoint");
        let run = f
            .runtime()
            .read::<AiRun>(&service(), "run-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(run.expires_at_unix_ms(), 301000);
        assert_eq!(run.state(), &RunState::AwaitingReconciliation);
        let budget = f
            .runtime()
            .read::<AiBudget>(&service(), "window-1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(budget.reserved(), reservation.entry().maximum_cost());
        assert!(
            f.runtime()
                .execute(
                    &service(),
                    Command::action("run-1", START_RUN, StartRun::new(1, 302001).unwrap())
                        .at_revision(4)
                        .idempotency("late-restart")
                )
                .await
                .is_err()
        );
    }
}
