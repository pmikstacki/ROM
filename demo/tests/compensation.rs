use rom::{Actor, Command, PrincipalKind, Resource, Runtime, StopReason, Storage, WorkState};
use rom_demo::{
    Notices,
    compensation::{self, Checkout, RECORD_PAYMENT, RESERVE, Stock},
    session_actor,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn storage(redb: bool, path: &Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
async fn drain(runtime: &Runtime) -> usize {
    let mut steps = 0;
    for _ in 0..8 {
        let n = runtime.process_work(16).await.unwrap();
        steps += n;
        if n == 0 {
            return steps;
        }
    }
    panic!("bounded work drain exhausted")
}
async fn case(redb: bool, revoked: bool) {
    let dir = Scratch(std::env::temp_dir().join(format!(
        "rom-demo-compensation-{}-{redb}-{revoked}",
        std::process::id()
    )));
    std::fs::create_dir_all(&dir.0).unwrap();
    let path = dir.0.join("db");
    let store = storage(redb, &path);
    let runtime = rom_demo::build(store.clone(), Notices::default()).unwrap();
    compensation::bootstrap(&runtime).await.unwrap();
    let actor = session_actor();
    assert_eq!(drain(&runtime).await, 1); // single checkout mapper, no target
    for (token, quantity, revision) in [("checkout-a", 3, 1), ("checkout-b", 2, 2)] {
        runtime
            .execute(
                &actor,
                Command::action(
                    "workshop-stock",
                    RESERVE,
                    BTreeMap::from([(token.into(), quantity)]),
                )
                .at_revision(revision)
                .idempotency(token),
            )
            .await
            .unwrap();
    }
    runtime
        .execute(
            &actor,
            Command::action("checkout-a", RECORD_PAYMENT, "unknown".into())
                .at_revision(1)
                .idempotency("unknown-a"),
        )
        .await
        .unwrap();
    assert_eq!(drain(&runtime).await, 1); // unknown does not schedule release
    let held = runtime
        .read::<Stock>(&actor, "workshop-stock")
        .await
        .unwrap();
    assert_eq!(held.value.unwrap().reservations.len(), 2);
    let before = runtime
        .journal(&actor, Stock::KIND, None)
        .await
        .unwrap()
        .events
        .len();
    assert_eq!(before, 3);
    if revoked {
        runtime.revoke(&Actor::trusted("demo-host", "worker").with_kind(PrincipalKind::Service));
    }
    let rejected = || {
        Command::action("checkout-a", RECORD_PAYMENT, "confirmed_rejected".into())
            .at_revision(2)
            .idempotency("rejected-a")
    };
    runtime.execute(&actor, rejected()).await.unwrap();
    assert_eq!(drain(&runtime).await, if revoked { 1 } else { 2 });
    runtime.execute(&actor, rejected()).await.unwrap();
    assert_eq!(drain(&runtime).await, 0);
    let current = runtime
        .read::<Stock>(&actor, "workshop-stock")
        .await
        .unwrap();
    let value = current.value.unwrap();
    assert_eq!(value.total, 10);
    assert_eq!(value.reservations.get("checkout-b"), Some(&2));
    assert_eq!(value.reservations.contains_key("checkout-a"), revoked);
    let after = runtime
        .journal(&actor, Stock::KIND, None)
        .await
        .unwrap()
        .events
        .len();
    assert_eq!(after - before, usize::from(!revoked));
    if revoked {
        assert!(
            store
                .reaction_records()
                .unwrap()
                .iter()
                .any(|r| r.state == WorkState::Stopped(StopReason::Denied))
        );
    }
    assert!(
        runtime
            .execute(
                &actor,
                Command::action("checkout-a", RECORD_PAYMENT, "succeeded".into())
                    .at_revision(3)
                    .idempotency("contradict-terminal")
            )
            .await
            .is_err()
    );
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(store);
    let reopened = rom_demo::build(storage(redb, &path), Notices::default()).unwrap();
    let stock = reopened
        .read::<Stock>(&actor, "workshop-stock")
        .await
        .unwrap();
    assert_eq!(stock.revision, current.revision);
    assert_eq!(
        stock.value.unwrap().reservations.get("checkout-b"),
        Some(&2)
    );
    let checkout = reopened
        .read::<Checkout>(&actor, "checkout-a")
        .await
        .unwrap();
    assert_eq!(
        checkout.value.unwrap().payment_outcome,
        "confirmed_rejected"
    );
    reopened.shutdown().await.unwrap();
}
#[tokio::test]
async fn explicit_compensation_and_revocation_on_both_adapters() {
    for redb in [false, true] {
        for revoked in [false, true] {
            case(redb, revoked).await;
        }
    }
}
