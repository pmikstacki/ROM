use super::*;
use crate::{compensation, reference, session_actor};
use rom::{Command, Shape, Storage, json};
use std::sync::Arc;

#[test]
fn frozen_checkout_preserves_the_original_string_codec() {
    let descriptor = CheckoutV1::descriptor();
    assert_eq!(descriptor.version, 1);
    assert_eq!(
        descriptor
            .fields
            .iter()
            .find(|field| field.name == "stock_id")
            .unwrap()
            .shape,
        Shape::String
    );
    // The old codec accepted an empty string; its transition rule rejected it.
    let empty = json!({"stock_id":"","reservation_id":"reservation","payment_outcome":"pending"});
    assert_eq!(CheckoutV1::decode(empty.clone()).unwrap().encode(), empty);
    assert!(Checkout::decode(empty).is_err());
    assert!(CheckoutV1::decode(json!({"stock_id":"stock","reservation_id":"reservation","payment_outcome":"pending","extra":true})).is_err());
}

#[tokio::test]
async fn current_wire_commands_prepare_the_frozen_profile_without_changing_its_catalog_or_rules() {
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = declarations(Notices::default())
        .unwrap()
        .build(store.clone(), rom::Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    reference::prepare(&runtime).await.unwrap();
    // Registration equality proves the persisted descriptor is still version one.
    store.register(&[CheckoutV1::descriptor()]).unwrap();
    assert!(store.register(&[Checkout::descriptor()]).is_err());
    let current = runtime
        .read::<CheckoutV1>(&session_actor(), "checkout-a")
        .await
        .unwrap();
    assert_eq!(current.revision, 3);
    assert_eq!(current.value.unwrap().payment_outcome, "confirmed_rejected");
    for (name, value, expected_field) in [
        (
            "payment_outcome",
            "succeeded",
            "terminal outcome cannot change",
        ),
        ("stock_id", "different", "compensation context"),
        ("reservation_id", "different", "compensation context"),
    ] {
        let operation = rom::Operation::Patch(std::collections::BTreeMap::from([(
            name.into(),
            rom::FieldUpdate::Set(json!(value)),
        )]));
        let invocation = rom::Invocation {
            retry_epoch: 0,
            kind: CheckoutV1::KIND.into(),
            id: "checkout-a".into(),
            expected: Some(3),
            idempotency: name.into(),
            operation,
        };
        assert_eq!(
            runtime.invoke(&session_actor(), invocation).await,
            Err(Error::invalid(CheckoutV1::KIND, expected_field))
        );
    }
    assert!(
        runtime
            .execute(
                &session_actor(),
                Command::<CheckoutV1>::delete("checkout-a")
                    .at_revision(3)
                    .idempotency("delete")
            )
            .await
            .is_err()
    );
    let unfinished: Vec<_> = store
        .reaction_records()
        .unwrap()
        .into_iter()
        .filter(|record| record.state != rom::WorkState::Done)
        .collect();
    assert_eq!(unfinished.len(), 1);
    validate_work(&unfinished[0].pending).unwrap();
    runtime.shutdown().await.unwrap();
}

fn row<R: Resource>(value: R) -> Row {
    Row {
        key: rom::Key {
            kind: R::KIND.into(),
            id: "source".into(),
        },
        revision: 1,
        value: Some(value.encode()),
        protected: Default::default(),
    }
}
fn work(definition: &str, payload: WorkPayload) -> PendingWork {
    let actor = service();
    PendingWork {
        delivery_profile: rom::DeliveryProfile::AtLeastOnce,
        id: "pending".into(),
        definition: definition.into(),
        version: 1,
        service_key: json!([actor.authority, actor.principal_kind(), actor.subject]).to_string(),
        cause: rom::Cause {
            retry_epoch: 0,
            root: "root".into(),
            parent: None,
            depth: 1,
            started_at: 0,
            path: vec![],
        },
        payload,
    }
}
fn action(kind: &str, name: &str, input: rom::Value) -> WorkPayload {
    WorkPayload::Action(
        serde_json::to_value(Invocation {
            retry_epoch: 0,
            kind: kind.into(),
            id: "target".into(),
            expected: Some(1),
            idempotency: "pending".into(),
            operation: Operation::Action {
                name: name.into(),
                input,
            },
        })
        .unwrap(),
    )
}
#[test]
fn migration_accepts_only_frozen_callback_identity_and_payload_contracts() {
    let checkout = Checkout {
        stock_id: ResourceRef::new("stock").unwrap(),
        reservation_id: "reservation".into(),
        payment_outcome: "confirmed_rejected".into(),
    };
    let accepted = [
        work(
            "rejected-checkout-release",
            WorkPayload::Source(row(checkout)),
        ),
        work(
            "rejected-checkout-release",
            action(Stock::KIND, "release", json!("reservation")),
        ),
        work(
            "completed-task-dashboard",
            WorkPayload::Source(row(Task {
                title: "done".into(),
                done: true,
            })),
        ),
        work(
            "completed-task-dashboard",
            action(Dashboard::KIND, "display", json!("done")),
        ),
        work(
            "local-completions",
            WorkPayload::Notification {
                source: row(Dashboard {
                    latest: "done".into(),
                }),
                payload: json!("done"),
            },
        ),
    ];
    for original in accepted {
        validate_work(&original).unwrap();
        let mut changed = original.clone();
        changed.version = 2;
        assert!(validate_work(&changed).is_err());
        let mut changed = original.clone();
        changed.definition = "unknown".into();
        assert!(validate_work(&changed).is_err());
        let mut changed = original;
        changed.service_key = "other-worker".into();
        assert!(validate_work(&changed).is_err());
    }
    for payload in [
        action(Stock::KIND, "reserve", json!("reservation")),
        action(Stock::KIND, "release", json!(1)),
        action(Dashboard::KIND, "release", json!("reservation")),
    ] {
        assert!(validate_work(&work("rejected-checkout-release", payload)).is_err());
    }
    // Known definition names cannot reinterpret another Resource's frozen source.
    assert!(
        validate_work(&work(
            "rejected-checkout-release",
            WorkPayload::Source(row(Task {
                title: "wrong".into(),
                done: true
            }))
        ))
        .is_err()
    );
}

#[tokio::test]
async fn old_profile_keeps_checkout_validation_before_reference_migration() {
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = declarations(Notices::default())
        .unwrap()
        .build(store, rom::Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    compensation::bootstrap(&runtime).await.unwrap();
    let invalid = CheckoutV1 {
        stock_id: String::new(),
        reservation_id: "reservation".into(),
        payment_outcome: "pending".into(),
    };
    assert!(
        matches!(runtime.execute(&session_actor(),Command::create("invalid",invalid).idempotency("invalid")).await,Err(Error::Invalid{field,..}) if field=="checkout state")
    );
    runtime.shutdown().await.unwrap();
}
