use crate::{InventoryItem, studio_application};
use rom::{Command, Invocation, Operation, Resource, Value};
use std::sync::Arc;

#[tokio::test]
async fn inventory_restock_is_discovered_and_rejects_overflow_without_commit() {
    let database = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = studio_application::build(database.clone(), Arc::new(rom::SystemClock)).unwrap();
    let actor = studio_application::host_actor();
    let discovery = runtime.discover(&actor).await.unwrap();
    let inventory = discovery
        .resources
        .iter()
        .find(|r| r.kind == InventoryItem::KIND)
        .unwrap();
    assert!(
        inventory.actions.iter().any(|name| name == "restock"),
        "Inventory must declare the demo action through Resource metadata"
    );
    let item =
        InventoryItem::decode(rom::json!({"code":"STOCK-A","quantity":u64::MAX - 1})).unwrap();
    runtime
        .execute(&actor, Command::create("one", item).idempotency("create"))
        .await
        .unwrap();
    let action = |expected, key: &str| Invocation {
        retry_epoch: 0,
        kind: InventoryItem::KIND.into(),
        id: "one".into(),
        expected: Some(expected),
        idempotency: key.into(),
        operation: Operation::Action {
            name: "restock".into(),
            input: Value::from(1_u64),
        },
    };
    let first = runtime
        .invoke(&actor, action(1, "restock-once"))
        .await
        .unwrap();
    assert_eq!(first.revision, 2);
    assert_eq!(
        first.value.as_ref().unwrap()["quantity"],
        Value::from(u64::MAX)
    );
    let before = database.counts().unwrap();
    assert!(runtime.invoke(&actor, action(2, "overflow")).await.is_err());
    assert_eq!(database.counts().unwrap(), before);
    let replay = runtime
        .invoke(&actor, action(1, "restock-once"))
        .await
        .unwrap();
    assert_eq!(replay, first);
    assert_eq!(database.counts().unwrap(), before);
    runtime.shutdown().await.unwrap();
}
