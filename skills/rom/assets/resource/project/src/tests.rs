use crate::{Code, Inventory, Note, RESERVE};
use rom::{Actor, Command, Error, Field, Invocation, Operation, Resource, Runtime, json};
use rom_conformance::CodecCase;
use std::sync::Arc;

#[test]
fn explicit_codec_cases_preserve_canonical_values_and_reject_invalid_input() {
    rom_conformance::profile::require(1).unwrap();
    rom_conformance::field::codec(
        &[CodecCase {
            input: json!(" ab-1 "),
            canonical: json!("AB-1"),
            expected: Code::decode(json!("AB-1")).unwrap(),
        }],
        &[json!(""), json!("non-ascii-ß"), json!(false)],
    )
    .unwrap();
    assert_eq!(
        rom_conformance::profile::require(0).unwrap_err().case,
        "profile.version"
    );
}

#[tokio::test]
async fn action_rejection_and_exact_replay_preserve_atomic_results() {
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Inventory::definition()
                .action(RESERVE)
                .allow_all_fields()
                .policy(|_, _, _| true),
        )
        .resource(Note::definition().allow_all_fields().policy(|_, _, _| true))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("example-host", "owner");
    runtime
        .execute(
            &actor,
            Command::create(
                "one",
                Inventory {
                    code: Code::decode(json!(" ab-1 ")).unwrap(),
                    available: 3,
                },
            )
            .idempotency("create"),
        )
        .await
        .unwrap();
    let request = Command::action("one", RESERVE, 2_u64)
        .at_revision(1)
        .idempotency("reserve");
    let first = runtime.execute(&actor, request.clone()).await.unwrap();
    let counts = store.counts().unwrap();
    let replay = runtime.execute(&actor, request).await.unwrap();
    assert_eq!(first.revision, replay.revision);
    assert_eq!(first.value, replay.value);
    assert_eq!(store.counts().unwrap(), counts);
    let invalid_field = Invocation {
        retry_epoch: 0,
        kind: Inventory::KIND.into(),
        id: "invalid-field".into(),
        expected: None,
        idempotency: "invalid-field".into(),
        operation: Operation::Create(json!({"code":"", "available":3})),
    };
    assert!(matches!(
        runtime.invoke(&actor, invalid_field).await,
        Err(Error::Invalid { field, .. }) if field == "code"
    ));
    assert_eq!(store.counts().unwrap(), counts);
    assert_eq!(
        runtime
            .execute(
                &actor,
                Command::action("one", RESERVE, 1_u64)
                    .at_revision(1)
                    .idempotency("reserve")
            )
            .await
            .unwrap_err(),
        Error::IdentityMismatch
    );
    assert!(matches!(
        runtime
            .execute(
                &actor,
                Command::action("one", RESERVE, 0_u64)
                    .at_revision(2)
                    .idempotency("invalid")
            )
            .await,
        Err(Error::Denied)
    ));
    assert_eq!(store.counts().unwrap(), counts);
    let current = runtime
        .read::<Inventory>(&actor, "one")
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(current.code.encode(), json!("AB-1"));
    assert_eq!(current.available, 1);
    runtime.shutdown().await.unwrap();
}
