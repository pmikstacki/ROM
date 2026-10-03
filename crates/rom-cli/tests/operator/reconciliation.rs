//! A trusted verifier resolves uncertainty through the public HTTP and CLI path.
use super::{common::*, native_support::*};
use rom::operator::*;
use rom::{
    Actor, Command, DeliveryOutcome, DeliveryProfile, DeliveryVerification, PrincipalKind, Runtime,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn verified_reconciliation_completes_without_resending_and_replays_exactly() {
    for redb in [false, true] {
        let directory = Directory::new();
        let (store, counts) = native_store(&directory, redb);
        let sends = Arc::new(AtomicUsize::new(0));
        let checks = Arc::new(AtomicUsize::new(0));
        let sent = sends.clone();
        let checked = checks.clone();
        let runtime = declarations()
            .channel_with(
                NOTICE
                    .delivery_profile(DeliveryProfile::ReconcileBeforeRetry)
                    .verifier(move |lookup| {
                        assert!(!lookup.id.is_empty());
                        assert_eq!(
                            lookup.evidence_ref.as_deref(),
                            Some("opaque-lookup-reference")
                        );
                        let attempt = checked.fetch_add(1, Ordering::SeqCst);
                        async move {
                            if attempt == 0 {
                                DeliveryVerification::Unresolved
                            } else {
                                DeliveryVerification::Accepted {
                                    evidence: "PRIVATE-VERIFIED-EVIDENCE".into(),
                                }
                            }
                        }
                    }),
                Actor::trusted("local", "provider").with_kind(PrincipalKind::Service),
                move |_| {
                    sent.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Unknown }
                },
            )
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let actor = rom_demo::session_actor();
        runtime
            .execute(
                &actor,
                Command::create("one", Note { sent: false }).idempotency("create"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &actor,
                Command::action("one", SEND, ())
                    .at_revision(1)
                    .idempotency("send"),
            )
            .await
            .unwrap();
        assert_eq!(runtime.process_work(1).await.unwrap(), 1);
        let server = Server::new(runtime.clone()).await;
        let auth = directory.file("auth", "Demo local");
        let page = json(&run(&server.endpoint, Some(&auth), &["work", "list"], "").await);
        let view: WorkView = serde_json::from_value(page["records"][0].clone()).unwrap();
        assert_eq!(view.state, WorkStatus::AwaitingReconciliation);
        let before = counts();
        let mut request = WorkControlRequest {
            handle: view.handle.clone(),
            expected: view.version.clone(),
            key: "retry-held".into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        };
        let refused = run(
            &server.endpoint,
            Some(&auth),
            &["work", "retry", "--request-file", "-"],
            &serde_json::to_string(&request).unwrap(),
        )
        .await;
        assert_eq!(refused.status.code(), Some(5));
        assert!(refused.stdout.is_empty());
        request.key = "verify-held".into();
        request.operation = WorkControlOperation::Reconcile {
            evidence_ref: Some("opaque-lookup-reference".into()),
        };
        let file = directory.file("reconcile.json", serde_json::to_vec(&request).unwrap());
        let args = ["work", "reconcile", "--request-file", file.as_str()];
        let unresolved = json(&run(&server.endpoint, Some(&auth), &args, "").await);
        assert_eq!(unresolved["outcome"], "Unresolved");
        assert_eq!(
            unresolved["version"],
            serde_json::to_value(&view.version).unwrap()
        );
        assert_eq!(unresolved["replayed"], false);
        assert_eq!(runtime.process_work(1).await.unwrap(), 0);
        let completed = json(&run(&server.endpoint, Some(&auth), &args, "").await);
        assert_eq!(completed["outcome"], "Completed");
        assert_eq!(completed["replayed"], false);
        let replay = json(&run(&server.endpoint, Some(&auth), &args, "").await);
        let mut expected = completed.clone();
        expected["replayed"] = true.into();
        assert_eq!(replay, expected);
        let shown = json(
            &run(
                &server.endpoint,
                Some(&auth),
                &["work", "show", view.handle.as_str()],
                "",
            )
            .await,
        );
        assert_eq!(shown["state"], "Done");
        for output in [&page, &unresolved, &completed, &replay, &shown] {
            for secret in ["PRIVATE-NOTIFICATION-PAYLOAD", "PRIVATE-VERIFIED-EVIDENCE"] {
                assert!(!output.to_string().contains(secret));
            }
        }
        assert_eq!(runtime.process_work(1).await.unwrap(), 0);
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        assert_eq!(checks.load(Ordering::SeqCst), 2);
        assert_eq!(counts(), before);
        server.finish().await;
        runtime.shutdown().await.unwrap();
    }
}
