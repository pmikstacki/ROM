use rom::{Error, Invocation, json};
use rom_demo::{
    Notices,
    compensation::{self, Stock},
    session_actor,
};
use std::sync::Arc;

#[tokio::test]
async fn reservation_arguments_are_named_and_invalid_fields_do_not_commit() {
    let runtime = rom_demo::build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        Notices::default(),
    )
    .unwrap();
    compensation::bootstrap(&runtime).await.unwrap();
    let request = |input, key: &str| -> Invocation {
        serde_json::from_value(json!({
            "kind":"reservation-stock", "id":"workshop-stock", "expected":1,
            "idempotency":key,
            "operation":{"type":"action","input":{"name":"reserve","input":input}}
        }))
        .unwrap()
    };
    let actor = session_actor();
    let valid = request(
        json!({"token":"checkout-a","quantity":3}),
        "named-reservation",
    );
    let invalid = request(
        json!({"token":"checkout-a","quantity":"SECRET_INVALID_VALUE"}),
        "bad-reservation",
    );
    assert_eq!(
        runtime.invoke(&actor, invalid).await.unwrap_err(),
        Error::invalid("reservation-stock", "reserve.quantity")
    );
    assert_eq!(
        runtime
            .read::<Stock>(&actor, "workshop-stock")
            .await
            .unwrap()
            .revision,
        1
    );
    let row = runtime.invoke(&actor, valid.clone()).await.unwrap();
    assert_eq!(row.revision, 2);
    assert_eq!(row.value.unwrap()["reservations"], json!({"checkout-a":3}));
    assert_eq!(runtime.invoke(&actor, valid).await.unwrap().revision, 2);
    runtime.shutdown().await.unwrap();
}
