use super::support::*;
use rom::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test]
async fn expired_started_claim_holds_after_clean_reopen_and_late_finish_is_fenced() {
    for redb in [false, true] {
        let mut fixture = Fixture::new(redb);
        let runtime = fixture.runtime(
            MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry),
            SendMode::Unknown,
        );
        enqueue(&runtime).await;
        let claim = match fixture
            .storage()
            .reaction_update(WorkUpdate::Claim { now: 100 })
            .unwrap()
        {
            WorkResult::Claimed(claim) => claim,
            _ => panic!("claim missing"),
        };
        fixture
            .storage()
            .reaction_update(WorkUpdate::DeliveryStarted {
                claim: claim.key(),
                now: 100,
            })
            .unwrap();
        let before = fixture.snapshot();
        shutdown(runtime).await;
        fixture.reopen();
        let runtime = fixture.runtime(
            MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry),
            SendMode::Accepted,
        );
        assert_eq!(runtime.process_work(8).await.unwrap(), 0);
        assert_eq!(fixture.snapshot(), before);
        fixture.clock.0.store(131, Ordering::SeqCst);
        assert_eq!(runtime.process_work(8).await.unwrap(), 0);
        let after = fixture.snapshot();
        assert_eq!(after.records[0].state, WorkState::AwaitingReconciliation);
        assert_eq!(after.records[0].attempts, before.records[0].attempts);
        assert_eq!(after.roots, before.roots);
        assert_eq!(after.records[0].revision, before.records[0].revision + 1);
        assert!(after.records[0].generation > claim.work.generation);
        assert_eq!(fixture.send_count(), 0);
        assert_eq!(
            fixture
                .storage()
                .reaction_update(WorkUpdate::DeliveryFinished {
                    claim: claim.key(),
                    now: 131,
                    outcome: DeliveryOutcome::Accepted
                }),
            Err(Error::Conflict)
        );
        shutdown(runtime).await;
    }
}

#[tokio::test]
async fn held_state_and_accepted_receipt_survive_clean_native_reopen() {
    for redb in [false, true] {
        let mut fixture = Fixture::new(redb);
        let calls = Arc::new(AtomicUsize::new(0));
        let registration = || {
            let calls = calls.clone();
            MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry)
                .verifier(move |_: DeliveryReconciliation| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    async {
                        DeliveryVerification::Accepted {
                            evidence: "durable-provider-reference".into(),
                        }
                    }
                })
        };
        let runtime = fixture.runtime(registration(), SendMode::Unknown);
        hold(&fixture, &runtime).await;
        let before = fixture.snapshot();
        shutdown(runtime).await;
        fixture.reopen();
        assert_eq!(fixture.snapshot(), before);
        let runtime = fixture.runtime(registration(), SendMode::Accepted);
        assert_eq!(runtime.process_work(8).await.unwrap(), 0);
        assert_eq!(fixture.send_count(), 1);
        let original = request(&fixture, "accepted-before-restart");
        let accepted = runtime
            .work_control(&actor(), original.clone())
            .await
            .unwrap();
        let saved = fixture.snapshot();
        shutdown(runtime).await;
        fixture.reopen();
        assert_eq!(fixture.snapshot(), saved);
        let runtime = fixture.runtime(registration(), SendMode::Accepted);
        let replay = runtime.work_control(&actor(), original).await.unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.version, accepted.version);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.send_count(), 1);
        shutdown(runtime).await;
    }
}

#[tokio::test]
async fn changed_registration_cannot_reinterpret_pending_or_held_profile() {
    for redb in [false, true] {
        for old in [
            DeliveryProfile::AtLeastOnce,
            DeliveryProfile::ReconcileBeforeRetry,
        ] {
            let mut fixture = Fixture::new(redb);
            let runtime = fixture.runtime(MAIL.delivery_profile(old.clone()), SendMode::Unknown);
            enqueue(&runtime).await;
            if old == DeliveryProfile::ReconcileBeforeRetry {
                runtime.process_work(8).await.unwrap();
            }
            let before = fixture.snapshot();
            shutdown(runtime).await;
            fixture.reopen();
            let changed = if old == DeliveryProfile::AtLeastOnce {
                DeliveryProfile::ReconcileBeforeRetry
            } else {
                DeliveryProfile::AtLeastOnce
            };
            let result = fixture
                .configure(
                    fixture.builder(),
                    MAIL.delivery_profile(changed),
                    SendMode::Accepted,
                )
                .build(fixture.storage(), Runtime::shared_cpu_pool(2).unwrap());
            match result {
                Err(Error::Unsupported(_)) => {}
                Err(other) => panic!("unexpected registration error: {other:?}"),
                Ok(runtime) => {
                    let req = request(&fixture, "profile-change");
                    assert!(matches!(
                        runtime.work_control(&actor(), req).await,
                        Err(Error::Unsupported(_))
                    ));
                    runtime.process_work(8).await.unwrap();
                    assert_eq!(
                        fixture.send_count(),
                        usize::from(old == DeliveryProfile::ReconcileBeforeRetry)
                    );
                    assert_ne!(fixture.snapshot().records[0].state, WorkState::Done);
                    shutdown(runtime).await;
                }
            }
            assert_eq!(
                fixture.snapshot().records[0].pending,
                before.records[0].pending
            );
        }
    }
}

#[tokio::test]
async fn absent_verifier_reports_unavailable_without_releasing_a_hold() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let runtime = fixture.runtime(
            MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry),
            SendMode::Unknown,
        );
        hold(&fixture, &runtime).await;
        let before = fixture.snapshot();
        assert!(matches!(
            runtime
                .work_control(&actor(), request(&fixture, "no-verifier"))
                .await,
            Err(Error::Unsupported(_))
        ));
        assert_eq!(fixture.snapshot(), before);
        assert_eq!(fixture.send_count(), 1);
        shutdown(runtime).await;
    }
}

#[path = "../support/child_process.rs"]
mod child_process;
use child_process::Process;

#[tokio::test]
async fn delivery_owner_child() {
    let Some(root) = std::env::var_os("ROM_RECONCILIATION_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let redb = std::env::var("ROM_RECONCILIATION_BACKEND").unwrap() == "redb";
    let fixture = std::mem::ManuallyDrop::new(Fixture {
        db: Some(Db::open(redb, &root.join("database"))),
        root: root.clone(),
        redb,
        clock: Arc::new(TestClock(std::sync::atomic::AtomicU64::new(100))),
        sends: Arc::new(std::sync::Mutex::new(vec![])),
        revoked: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let channel = MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry);
    let runtime = fixture
        .builder()
        .delivery_timeout(Duration::from_secs(20))
        .channel_with(channel, service(), move |delivery: Delivery<String>| {
            let root = root.clone();
            async move {
                std::fs::write(root.join("delivery-id"), delivery.id).unwrap();
                std::fs::write(root.join("ready"), b"delivery-started").unwrap();
                std::future::pending::<DeliveryOutcome>().await
            }
        })
        .build(fixture.storage(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    enqueue(&runtime).await;
    runtime.process_work(1).await.unwrap();
    panic!("parent must interrupt provider call");
}

#[tokio::test]
async fn actual_process_exit_after_delivery_start_cannot_trigger_unsafe_resend() {
    for redb in [false, true] {
        let mut fixture = Fixture::new(redb);
        fixture.close();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "recovery::delivery_owner_child", "--nocapture"])
            .env("ROM_RECONCILIATION_ROOT", &fixture.root)
            .env(
                "ROM_RECONCILIATION_BACKEND",
                if redb { "redb" } else { "sqlite" },
            )
            .stdin(std::process::Stdio::null());
        let mut child = Process::spawn(command);
        child.wait_ready(&fixture.root.join("ready"));
        let sent_id = std::fs::read_to_string(fixture.root.join("delivery-id")).unwrap();
        let status = child.kill();
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(status.signal(), Some(9));
        }
        fixture.reopen();
        fixture.clock.0.store(131, Ordering::SeqCst);
        let runtime = fixture.runtime(
            MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry),
            SendMode::Accepted,
        );
        assert_eq!(runtime.process_work(8).await.unwrap(), 0);
        let after = fixture.snapshot();
        assert_eq!(after.records[0].pending.id, sent_id);
        assert_eq!(after.records[0].state, WorkState::AwaitingReconciliation);
        assert_eq!(after.records[0].attempts, 1);
        assert_eq!(fixture.send_count(), 0);
        assert!(after.operator.receipts.is_empty());
        shutdown(runtime).await;
    }
}
