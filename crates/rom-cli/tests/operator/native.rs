//! Public Resource -> durable work -> HTTP -> CLI on both supported native stores.
use super::common::*;
use super::native_support::*;
use rom::operator::*;
use rom::{Actor, DeliveryOutcome, PrincipalKind, Runtime};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_operator_commands_inspect_retry_and_replay_without_domain_or_provider_effects() {
    for redb in [false, true] {
        let directory = Directory::new();
        let (store, counts) = native_store(&directory, redb);
        let sends = Arc::new(AtomicUsize::new(0));
        let sent = sends.clone();
        let runtime = declarations()
            .channel(
                NOTICE,
                Actor::trusted("local", "PRIVATE-PROVIDER-SERVICE")
                    .with_kind(PrincipalKind::Service),
                move |_| {
                    sent.fetch_add(1, Ordering::SeqCst);
                    async { DeliveryOutcome::Accepted }
                },
            )
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let server = Server::new(runtime.clone()).await;
        let auth = directory.file("auth", "Demo local");
        json(
            &run(
                &server.endpoint,
                Some(&auth),
                &[
                    "create",
                    "operator-notes",
                    "one",
                    "--idempotency",
                    "PRIVATE-DOMAIN-ROOT",
                    "--input-file",
                    "-",
                ],
                r#"{"sent":false}"#,
            )
            .await,
        );
        json(
            &run(
                &server.endpoint,
                Some(&auth),
                &[
                    "action",
                    "operator-notes",
                    "one",
                    "send",
                    "--expected",
                    "1",
                    "--idempotency",
                    "PRIVATE-ACTION-ROOT",
                    "--input-file",
                    "-",
                ],
                "null",
            )
            .await,
        );

        let capabilities =
            json(&run(&server.endpoint, Some(&auth), &["work", "capabilities"], "").await);
        assert_eq!(
            capabilities,
            json!({"protocol_version":1,"inspect":true,"retry":true,"reconcile":true})
        );
        let query = WorkQuery {
            state: None,
            category: None,
            definition: None,
            limit: 64,
            cursor: None,
        };
        let embedded = runtime
            .work_list(&rom_demo::session_actor(), query)
            .await
            .unwrap();
        assert_eq!(embedded.records.len(), 1);
        let listed = json(&run(&server.endpoint, Some(&auth), &["work", "list"], "").await);
        assert_eq!(listed, serde_json::to_value(&embedded).unwrap());
        let view = &embedded.records[0];
        let shown = json(
            &run(
                &server.endpoint,
                Some(&auth),
                &["work", "show", view.handle.as_str()],
                "",
            )
            .await,
        );
        assert_eq!(shown, serde_json::to_value(view).unwrap());
        for sentinel in [
            "PRIVATE-NOTIFICATION-PAYLOAD",
            "PRIVATE-PROVIDER-SERVICE",
            "PRIVATE-DOMAIN-ROOT",
            "PRIVATE-ACTION-ROOT",
        ] {
            assert!(!listed.to_string().contains(sentinel));
            assert!(!shown.to_string().contains(sentinel));
        }
        let before_counts = counts();
        let request = WorkControlRequest {
            handle: view.handle.clone(),
            expected: view.version.clone(),
            key: "native-retry".into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        };
        let file = directory.file("retry", serde_json::to_vec(&request).unwrap());
        let args = ["work", "retry", "--request-file", &file];
        let accepted = json(&run(&server.endpoint, Some(&auth), &args, "").await);
        assert_eq!(accepted["outcome"], "Scheduled");
        assert_eq!(accepted["replayed"], false);
        let replay = json(&run(&server.endpoint, Some(&auth), &args, "").await);
        let mut expected_replay = accepted.clone();
        expected_replay["replayed"] = json!(true);
        assert_eq!(replay, expected_replay);
        let after = json(
            &run(
                &server.endpoint,
                Some(&auth),
                &["work", "show", view.handle.as_str()],
                "",
            )
            .await,
        );
        assert_eq!(after["version"], accepted["version"]);
        assert_eq!(after["state"], "Pending");

        // This profile has no registered verifier. A reconciliation request is refused without a durable change.
        let before = store.work_snapshot(2048, 4 * 1024 * 1024).unwrap();
        let reconcile = WorkControlRequest {
            handle: view.handle.clone(),
            expected: serde_json::from_value(accepted["version"].clone()).unwrap(),
            key: "native-reconcile".into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Reconcile {
                evidence_ref: Some("PRIVATE-UNTRUSTED-EVIDENCE".into()),
            },
        };
        let out = run(
            &server.endpoint,
            Some(&auth),
            &["work", "reconcile", "--request-file", "-"],
            &serde_json::to_string(&reconcile).unwrap(),
        )
        .await;
        assert_eq!(out.status.code(), Some(5));
        assert!(out.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&out.stderr).contains("PRIVATE-UNTRUSTED-EVIDENCE"));
        assert_eq!(
            serde_json::to_value(store.work_snapshot(2048, 4 * 1024 * 1024).unwrap()).unwrap(),
            serde_json::to_value(before).unwrap()
        );
        assert_eq!(counts(), before_counts);
        assert_eq!(sends.load(Ordering::SeqCst), 0);
        assert_eq!(
            run(&server.endpoint, None, &["work", "list"], "")
                .await
                .status
                .code(),
            Some(3)
        );
        server.finish().await;
    }
}
