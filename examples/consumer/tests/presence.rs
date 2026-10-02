use rom::{
    Access, Action, Actor, Command, Error, Input, Patch, Presence, QuerySpec, Resource, Runtime,
    json,
};
use rom_sqlite::Sqlite;
use std::sync::Arc;
#[derive(Clone, Resource)]
#[resource(name = "notes")]
struct Note {
    title: String,
    memo: Presence<Option<String>>,
}
fn actor() -> Actor {
    Actor::trusted("host", "alice")
}
fn setup() -> Runtime {
    Runtime::builder()
        .resource(
            Note::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(SET),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap()
}
const SET: Action<Note, Presence<Option<String>>> = Action::new("memo", |n, p| {
    n.memo = p;
    Ok(vec![])
});
#[test]
fn presence_codec_distinguishes_missing_null_value_and_action_input() {
    for (memo, expected) in [
        (Presence::Missing, json!({"title":"a"})),
        (Presence::Value(None), json!({"title":"a","memo":null})),
        (
            Presence::Value(Some("b".into())),
            json!({"title":"a","memo":"b"}),
        ),
    ] {
        let n = Note {
            title: "a".into(),
            memo: memo.clone(),
        };
        assert_eq!(n.encode(), expected);
        assert_eq!(Note::decode(expected).unwrap().memo, memo);
        assert_eq!(
            <Presence<Option<String>> as Input>::decode(Input::encode(&memo)).unwrap(),
            memo
        );
    }
}
#[tokio::test]
async fn patch_preserves_omitted_fields_null_removal_retry_and_live_query() {
    let r = setup();
    r.execute(
        &actor(),
        Command::create(
            "n",
            Note {
                title: "a".into(),
                memo: Presence::Value(Some("b".into())),
            },
        )
        .idempotency("c"),
    )
    .await
    .unwrap();
    let mut live = r
        .live(&actor(), Note::memo_field().equals(Presence::Missing))
        .await
        .unwrap();
    assert!(live.changed().await.unwrap().is_empty());
    let p = Command::patch(
        "n",
        Patch::new().set(Note::memo_field(), Presence::Value(None)),
    )
    .at_revision(1)
    .idempotency("null");
    let n = r.execute(&actor(), p.clone()).await.unwrap();
    assert_eq!(n.revision, 2);
    assert_eq!(n.value.unwrap().memo, Presence::Value(None));
    assert_eq!(r.execute(&actor(), p).await.unwrap().revision, 2);
    let n = r
        .execute(
            &actor(),
            Command::patch("n", Patch::new().remove(Note::memo_field()))
                .at_revision(2)
                .idempotency("remove"),
        )
        .await
        .unwrap();
    assert_eq!(n.value.unwrap().encode(), json!({"title":"a"}));
    assert_eq!(live.changed().await.unwrap().len(), 1);
    assert!(
        r.query_spec_projected(&actor(), "notes", QuerySpec::equal("memo", json!(null)))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        r.query_spec_projected(&actor(), "notes", QuerySpec::absent("title"))
            .await
            .is_err()
    );
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn complete_reads_cannot_reveal_that_a_forbidden_field_is_absent() {
    let r = Runtime::builder()
        .resource(
            Note::definition()
                .policy(|_, _, _| true)
                .field_policy(|a, access, f, _| {
                    a.subject == "alice" || (matches!(access, Access::Read) && f == "title")
                }),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &actor(),
        Command::create(
            "n",
            Note {
                title: "a".into(),
                memo: Presence::Missing,
            },
        )
        .idempotency("c"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.read::<Note>(&Actor::trusted("host", "bob"), "n").await,
        Err(Error::Denied)
    ));
    assert_eq!(
        r.read_projected(&Actor::trusted("host", "bob"), "notes", "n")
            .await
            .unwrap()
            .value
            .unwrap()
            .len(),
        1
    );
    r.shutdown().await.unwrap();
}
#[derive(Clone, Resource)]
#[resource(name = "ambiguous_presence")]
struct Ambiguous {
    values: Vec<Presence<String>>,
}
#[test]
fn nested_presence_is_rejected_at_registration() {
    assert!(
        Runtime::builder()
            .resource(Ambiguous::definition())
            .build(
                Arc::new(Sqlite::open(":memory:").unwrap()),
                Runtime::shared_cpu_pool(1).unwrap()
            )
            .is_err()
    );
}
#[tokio::test]
async fn wire_patch_rejects_required_removal_and_action_preserves_explicit_missing() {
    use rom::{FieldUpdate, Invocation, Operation};
    use std::collections::BTreeMap;
    let r = setup();
    r.execute(
        &actor(),
        Command::create(
            "n",
            Note {
                title: "a".into(),
                memo: Presence::Value(None),
            },
        )
        .idempotency("c"),
    )
    .await
    .unwrap();
    let invalid = Invocation {
        kind: "notes".into(),
        id: "n".into(),
        expected: Some(1),
        idempotency: "bad".into(),
        operation: Operation::Patch(BTreeMap::from([("title".into(), FieldUpdate::Remove)])),
    };
    assert!(matches!(
        r.invoke(&actor(), invalid).await,
        Err(Error::Invalid { .. })
    ));
    let changed = r
        .execute(
            &actor(),
            Command::action("n", SET, Presence::Missing)
                .at_revision(1)
                .idempotency("missing"),
        )
        .await
        .unwrap();
    assert_eq!(changed.value.unwrap().memo, Presence::Missing);
    let noop = r
        .execute(
            &actor(),
            Command::patch("n", Patch::<Note>::new())
                .at_revision(2)
                .idempotency("noop"),
        )
        .await
        .unwrap();
    assert_eq!(noop.revision, 2);
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn forbidden_unchanged_patch_field_is_rejected_without_blocking_other_fields() {
    let r = Runtime::builder()
        .resource(
            Note::definition()
                .policy(|_, _, _| true)
                .field_policy(|a, access, f, _| {
                    a.subject == "alice" || matches!(access, Access::Read) || f == "title"
                }),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &actor(),
        Command::create(
            "n",
            Note {
                title: "a".into(),
                memo: Presence::Missing,
            },
        )
        .idempotency("c"),
    )
    .await
    .unwrap();
    let bob = Actor::trusted("host", "bob");
    assert!(matches!(
        r.execute(
            &bob,
            Command::patch("n", Patch::new().remove(Note::memo_field()))
                .at_revision(1)
                .idempotency("denied")
        )
        .await,
        Err(Error::Denied)
    ));
    assert_eq!(
        r.execute(
            &bob,
            Command::patch("n", Patch::new().set(Note::title_field(), "b".into()))
                .at_revision(1)
                .idempotency("allowed")
        )
        .await
        .unwrap()
        .value
        .unwrap()
        .title,
        "b"
    );
    r.shutdown().await.unwrap();
}
