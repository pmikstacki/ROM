use rom::{Command, Patch, Resource, Storage};
use rom_demo::{
    Notices,
    compensation::{self, Checkout, RECORD_PAYMENT},
    session_actor,
};
use std::sync::Arc;

#[tokio::test]
async fn generic_patch_cannot_reverse_a_confirmed_checkout() {
    let storage: Arc<dyn Storage> = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = rom_demo::build(storage.clone(), Notices::default()).unwrap();
    compensation::bootstrap(&runtime).await.unwrap();
    let actor = session_actor();
    runtime
        .execute(
            &actor,
            Command::action("checkout-a", RECORD_PAYMENT, "confirmed_rejected".into())
                .at_revision(1)
                .idempotency("reject"),
        )
        .await
        .unwrap();
    let before = runtime
        .journal(&actor, Checkout::KIND, None)
        .await
        .unwrap()
        .events
        .len();
    let work_before = storage.reaction_records().unwrap().len();
    let patch = Patch::new().set(Checkout::payment_outcome_field(), "succeeded".into());
    assert!(
        runtime
            .execute(
                &actor,
                Command::patch("checkout-a", patch)
                    .at_revision(2)
                    .idempotency("bypass")
            )
            .await
            .is_err(),
        "generic patch bypassed terminal outcome"
    );
    assert_eq!(
        runtime
            .read::<Checkout>(&actor, "checkout-a")
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        runtime
            .journal(&actor, Checkout::KIND, None)
            .await
            .unwrap()
            .events
            .len(),
        before
    );
    assert_eq!(storage.reaction_records().unwrap().len(), work_before);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn transported_mutations_preserve_demo_invariants_on_both_adapters() {
    use rom::{Invocation, json};
    use rom_demo::compensation::Stock;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::oneshot,
    };
    for redb in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "rom-demo-transition-http-{}-{redb}",
            std::process::id()
        ));
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let runtime = rom_demo::build(storage.clone(), Notices::default()).unwrap();
        compensation::bootstrap(&runtime).await.unwrap();
        let actor = session_actor();
        runtime
            .execute(
                &actor,
                Command::action("checkout-a", RECORD_PAYMENT, "confirmed_rejected".into())
                    .at_revision(1)
                    .idempotency("reject"),
            )
            .await
            .unwrap();
        let http =
            rom_http::Http::new(runtime.clone(), rom_demo::resolver(), Default::default()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = oneshot::channel();
        let server = tokio::spawn(http.serve(listener, async {
            let _ = stopped.await;
        }));
        let checkout = Command::replace(
            "checkout-a",
            Checkout {
                stock_id: "workshop-stock".into(),
                reservation_id: "checkout-a".into(),
                payment_outcome: "succeeded".into(),
            },
        )
        .at_revision(2)
        .idempotency("wire-replace");
        let stock = Command::replace(
            "workshop-stock",
            Stock {
                total: 1,
                reservations: std::collections::BTreeMap::from([("overbooked".into(), 2)]),
            },
        )
        .at_revision(1)
        .idempotency("wire-overbook");
        let work_before = storage.reaction_records().unwrap().len();
        for invocation in [Invocation::from(checkout), Invocation::from(stock)] {
            let body = serde_json::to_string(&invocation).unwrap();
            let mut socket = TcpStream::connect(address).await.unwrap();
            socket.write_all(format!("POST /invoke HTTP/1.1\r\nHost: localhost\r\nAuthorization: Demo local\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            let mut response = String::new();
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                socket.read_to_string(&mut response),
            )
            .await
            .unwrap()
            .unwrap();
            assert!(response.starts_with("HTTP/1.1 400"), "{response}");
        }
        assert_eq!(
            runtime
                .read::<Checkout>(&actor, "checkout-a")
                .await
                .unwrap()
                .revision,
            2
        );
        assert_eq!(
            runtime
                .read::<Stock>(&actor, "workshop-stock")
                .await
                .unwrap()
                .revision,
            1
        );
        assert_eq!(
            storage
                .journal(Checkout::KIND, None, 100, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
        assert_eq!(
            storage
                .journal(Stock::KIND, None, 100, 100_000)
                .unwrap()
                .events
                .len(),
            1
        );
        assert_eq!(storage.reaction_records().unwrap().len(), work_before);
        assert_eq!(
            runtime
                .read::<Stock>(&actor, "workshop-stock")
                .await
                .unwrap()
                .value
                .unwrap()
                .encode()["reservations"],
            json!({})
        );
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        drop(runtime);
        drop(storage);
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn generic_writes_cannot_retarget_compensation_or_corrupt_reservations() {
    use rom_demo::compensation::{RESERVE, Stock};
    use std::collections::BTreeMap;
    for redb in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "rom-demo-transition-state-{}-{redb}",
            std::process::id()
        ));
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let runtime = rom_demo::build(storage.clone(), Notices::default()).unwrap();
        compensation::bootstrap(&runtime).await.unwrap();
        let actor = session_actor();
        runtime
            .execute(
                &actor,
                Command::action(
                    "workshop-stock",
                    RESERVE,
                    BTreeMap::from([("checkout-a".into(), 2)]),
                )
                .at_revision(1)
                .idempotency("reserve"),
            )
            .await
            .unwrap();
        let work_before = storage.reaction_records().unwrap().len();
        for (n, patch) in [
            Patch::new().set(Checkout::stock_id_field(), "another-stock".into()),
            Patch::new().set(Checkout::reservation_id_field(), "another-checkout".into()),
            Patch::new().set(Checkout::payment_outcome_field(), "unrecognized".into()),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                runtime
                    .invoke(
                        &actor,
                        Command::patch("checkout-a", patch)
                            .at_revision(1)
                            .idempotency(&format!("context-{n}"))
                            .into()
                    )
                    .await
                    .is_err()
            );
        }
        for (n, invalid) in [
            Stock {
                total: 1,
                reservations: BTreeMap::from([("checkout-a".into(), 2)]),
            },
            Stock {
                total: 10,
                reservations: BTreeMap::from([("".into(), 2)]),
            },
            Stock {
                total: 10,
                reservations: BTreeMap::from([("checkout-a".into(), 0)]),
            },
            Stock {
                total: u64::MAX,
                reservations: BTreeMap::from([("a".into(), u64::MAX), ("b".into(), 1)]),
            },
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                runtime
                    .execute(
                        &actor,
                        Command::replace("workshop-stock", invalid)
                            .at_revision(2)
                            .idempotency(&format!("stock-{n}"))
                    )
                    .await
                    .is_err()
            );
        }
        assert!(
            runtime
                .execute(
                    &actor,
                    Command::<Stock>::delete("workshop-stock")
                        .at_revision(2)
                        .idempotency("stock-delete")
                )
                .await
                .is_err()
        );
        assert_eq!(
            runtime
                .read::<Stock>(&actor, "workshop-stock")
                .await
                .unwrap()
                .revision,
            2
        );
        assert_eq!(
            runtime
                .read::<Checkout>(&actor, "checkout-a")
                .await
                .unwrap()
                .revision,
            1
        );
        assert_eq!(
            storage
                .journal(Stock::KIND, None, 100, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
        assert_eq!(
            storage
                .journal(Checkout::KIND, None, 100, 100_000)
                .unwrap()
                .events
                .len(),
            1
        );
        assert_eq!(storage.reaction_records().unwrap().len(), work_before);
        runtime
            .execute(
                &actor,
                Command::action("checkout-a", RECORD_PAYMENT, "succeeded".into())
                    .at_revision(1)
                    .idempotency("succeed"),
            )
            .await
            .unwrap();
        assert!(
            runtime
                .execute(
                    &actor,
                    Command::<Checkout>::delete("checkout-a")
                        .at_revision(2)
                        .idempotency("checkout-delete")
                )
                .await
                .is_err()
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(storage);
        std::fs::remove_file(path).unwrap();
    }
}
