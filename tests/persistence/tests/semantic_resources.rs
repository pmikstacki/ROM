//! Real database stories for the standard semantic fields, through generic ROM APIs.
use rom::{
    Action, Actor, Command, CompareOp, Direction, Error, FieldUpdate, Input, Invocation, Key,
    Operation, Presence, QuerySpec, Resource, Runtime, Storage, Value, json,
};
use rom_fields::{
    Color, Date, DateTime, Decimal, Email, JsonDocument, Multiline, Time, UnitValue, Url,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Resource)]
#[resource(
    name = "semantic-stories",
    label = "Measurements",
    title_field = "title"
)]
struct Measurement {
    owner: String,
    title: String,
    secret: String,
    day: Date,
    time: Time,
    instant: DateTime,
    color: Color,
    email: Email,
    url: Url,
    body: Multiline,
    document: JsonDocument,
    amount: Decimal,
    unit: UnitValue,
    optional: Presence<Option<Date>>,
    dates: Vec<Date>,
    amounts: BTreeMap<String, Option<Decimal>>,
}
#[derive(Clone, Resource)]
#[resource(name = "ordinary-stories", label = "Equipment", title_field = "name")]
struct Equipment {
    name: String,
    enabled: bool,
    count: u64,
}
#[derive(Clone, Input)]
struct CorrectInput {
    amount: Decimal,
    instant: DateTime,
    document: JsonDocument,
}
const CORRECT: Action<Measurement, CorrectInput> = Action::new("correct", |row, input| {
    row.amount = input.amount;
    row.instant = input.instant;
    row.document = input.document;
    Ok(vec![])
});

trait Inspect: Storage {
    fn counts(&self) -> [u64; 4];
}
impl Inspect for rom_sqlite::Sqlite {
    fn counts(&self) -> [u64; 4] {
        self.counts().unwrap()
    }
}
impl Inspect for rom_redb::Redb {
    fn counts(&self) -> [u64; 4] {
        self.counts().unwrap()
    }
}
struct Fixture {
    path: PathBuf,
    redb: bool,
}
impl Fixture {
    fn new(redb: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-semantic-story-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self { path, redb }
    }
    fn open(&self) -> Arc<dyn Inspect> {
        let path = self.path.join("database");
        if self.redb {
            Arc::new(rom_redb::Redb::open(path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
fn runtime(store: Arc<dyn Inspect>) -> Runtime {
    Runtime::builder()
        .resource(
            Measurement::definition()
                .policy(|actor, _, row| actor.subject == "admin" || actor.subject == row.owner)
                .field_policy(|actor, _, field, _| actor.subject == "admin" || field != "secret")
                .query_policy(|actor, field| actor.subject == "admin" || field != "secret")
                .sort_policy(|actor, field| actor.subject == "admin" || field != "secret")
                .action(CORRECT),
        )
        .resource(
            Equipment::definition()
                .policy(|actor, _, _| actor.subject == "admin")
                .allow_all_fields(),
        )
        .build(store, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
fn actor(subject: &str) -> Actor {
    Actor::trusted("semantic-tests", subject)
}
fn key(id: &str) -> Key {
    Key {
        kind: Measurement::KIND.into(),
        id: id.into(),
    }
}
fn invoke(id: &str, expected: Option<u64>, identity: &str, operation: Operation) -> Invocation {
    Invocation {
        retry_epoch: 0,
        kind: Measurement::KIND.into(),
        id: id.into(),
        expected,
        idempotency: identity.into(),
        operation,
    }
}
const EXACT_JSON: &str = " {\"n\":123456789012345678901234567890.12345678901234567890} ";
fn candidate(amount: &str, instant: &str) -> Value {
    json!({"owner":"alice","title":"Sample","secret":"HIDDEN-SEMANTIC-STORY",
        "day":"2000-02-29","time":"12:30:10.12000","instant":instant,
        "color":"#ABCdefFF","email":"Alice+tag@EXAMPLE.TEST","url":"HTTPS://EXAMPLE.TEST:443/a",
        "body":"one\r\ntwo\n","document":EXACT_JSON,"amount":amount,
        "unit":{"value":"001.20","unit":"kg"},"dates":["0001-01-01","9999-12-31"],
        "amounts":{"exact":"00012345678901234567890.012300","empty":null}})
}
fn patch(entries: &[(&str, FieldUpdate)]) -> Operation {
    Operation::Patch(
        entries
            .iter()
            .map(|(name, update)| ((*name).into(), update.clone()))
            .collect(),
    )
}
async fn rejected(runtime: &Runtime, store: &dyn Inspect, request: Invocation) {
    let before = store.load(&key("a")).unwrap();
    let counts = store.counts();
    assert!(matches!(
        runtime.invoke(&actor("admin"), request).await,
        Err(Error::Invalid { .. })
    ));
    assert_eq!(store.load(&key("a")).unwrap(), before);
    assert_eq!(store.counts(), counts);
}

#[tokio::test]
async fn semantic_create_patch_action_authority_and_restart_on_both_databases() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let store = fixture.open();
        let runtime = runtime(store.clone());
        let create = invoke(
            "a",
            None,
            "create-a",
            Operation::Create(candidate("0002.000", "2024-03-31T01:30:00+01:00")),
        );
        let created = runtime
            .invoke(&actor("admin"), create.clone())
            .await
            .unwrap();
        assert_eq!(created.revision, 1);
        let canonical = store.load(&key("a")).unwrap().unwrap().value.unwrap();
        assert_eq!(canonical["time"], "12:30:10.12");
        assert_eq!(canonical["instant"], "2024-03-31T00:30:00.000000000Z");
        assert_eq!(canonical["amount"], "2");
        assert_eq!(canonical["color"], "#abcdefff");
        assert_eq!(canonical["email"], "Alice+tag@example.test");
        assert_eq!(canonical["url"], "https://example.test/a");
        assert_eq!(canonical["document"], EXACT_JSON);
        assert_eq!(canonical["body"], "one\r\ntwo\n");
        assert_eq!(canonical["amounts"]["exact"], "12345678901234567890.0123");
        assert_eq!(canonical["unit"], json!({"value":"1.2","unit":"kg"}));
        assert!(canonical.get("optional").is_none());
        let before_retry = store.counts();
        assert_eq!(
            runtime
                .invoke(&actor("admin"), create.clone())
                .await
                .unwrap(),
            created
        );
        assert_eq!(store.counts(), before_retry);
        let equivalent = invoke("a", None, "create-a", Operation::Create(canonical.clone()));
        assert_eq!(
            runtime.invoke(&actor("admin"), equivalent).await.unwrap(),
            created
        );
        assert_eq!(store.counts(), before_retry);

        let mut second = candidate("10", "2024-03-31T01:30:00Z");
        second["optional"] = json!("2000-02-29");
        runtime
            .invoke(
                &actor("admin"),
                invoke("b", None, "create-b", Operation::Create(second)),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &actor("admin"),
                Command::create(
                    "equipment",
                    Equipment {
                        name: "Pump".into(),
                        enabled: false,
                        count: 0,
                    },
                )
                .idempotency("equipment"),
            )
            .await
            .unwrap();
        assert_eq!(store.counts(), [3, 3, 3, 0]);
        let equipment = runtime
            .read::<Equipment>(&actor("admin"), "equipment")
            .await
            .unwrap()
            .value
            .unwrap();
        assert!(!equipment.enabled);
        assert_eq!(equipment.count, 0);

        let same = runtime
            .query_spec_projected(
                &actor("admin"),
                Measurement::KIND,
                QuerySpec::equal("instant", json!("2024-03-31T02:30:00+02:00")),
            )
            .await
            .unwrap();
        assert_eq!(
            same.iter().map(|r| r.key.id.as_str()).collect::<Vec<_>>(),
            ["a"]
        );
        let later = runtime
            .query_spec_projected(
                &actor("admin"),
                Measurement::KIND,
                QuerySpec::all().compare(
                    "instant",
                    CompareOp::Gt,
                    json!("2024-03-31T01:30:00+01:00"),
                ),
            )
            .await
            .unwrap();
        assert_eq!(
            later.iter().map(|r| r.key.id.as_str()).collect::<Vec<_>>(),
            ["b"]
        );
        // Decimal is exact text. The generic string order is deliberately lexical.
        let lexical = runtime
            .query_spec_projected(
                &actor("admin"),
                Measurement::KIND,
                QuerySpec::all().order_by("amount", Direction::Asc),
            )
            .await
            .unwrap();
        assert_eq!(
            lexical
                .iter()
                .map(|r| r.key.id.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );

        for (field, invalid) in [
            ("day", json!("1900-02-29")),
            ("instant", json!("2024-03-31T01:30:00-00:00")),
            ("amount", json!(1.2)),
            ("color", json!("#abc")),
            ("email", json!("a..b@example.test")),
            ("url", json!("javascript:alert(1)")),
            ("document", json!("[1,]")),
            ("dates", json!(["2025-02-29"])),
            ("amounts", json!({"bad":123})),
            ("optional", json!("2025-02-29")),
            ("unit", json!({"value":1,"unit":"kg"})),
        ] {
            rejected(
                &runtime,
                store.as_ref(),
                invoke(
                    "a",
                    Some(1),
                    field,
                    patch(&[
                        ("title", FieldUpdate::Set(json!("must rollback"))),
                        (field, FieldUpdate::Set(invalid)),
                    ]),
                ),
            )
            .await;
        }
        let mut invalid_create = candidate("1", "2024-03-31T00:30:00Z");
        invalid_create["day"] = json!("2025-02-29");
        rejected(
            &runtime,
            store.as_ref(),
            invoke(
                "invalid",
                None,
                "bad-create",
                Operation::Create(invalid_create),
            ),
        )
        .await;
        assert!(store.load(&key("invalid")).unwrap().is_none());

        let null_patch = invoke(
            "a",
            Some(1),
            "null",
            patch(&[
                ("optional", FieldUpdate::Set(Value::Null)),
                ("color", FieldUpdate::Set(json!("#AABBCC"))),
            ]),
        );
        assert_eq!(
            runtime
                .invoke(&actor("admin"), null_patch.clone())
                .await
                .unwrap()
                .revision,
            2
        );
        assert_eq!(
            store.load(&key("a")).unwrap().unwrap().value.unwrap()["optional"],
            Value::Null
        );
        let counts = store.counts();
        assert_eq!(counts, [3, 4, 4, 0]);
        runtime.invoke(&actor("admin"), null_patch).await.unwrap();
        assert_eq!(store.counts(), counts);
        runtime
            .invoke(
                &actor("admin"),
                invoke(
                    "a",
                    Some(2),
                    "remove",
                    patch(&[
                        ("optional", FieldUpdate::Remove),
                        (
                            "amounts",
                            FieldUpdate::Set(json!({"exact":"000.012300","empty":null})),
                        ),
                    ]),
                ),
            )
            .await
            .unwrap();
        assert!(
            !store
                .load(&key("a"))
                .unwrap()
                .unwrap()
                .value
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("optional")
        );

        let action = invoke(
            "a",
            Some(3),
            "correction",
            Operation::Action {
                name: "correct".into(),
                input: json!({"amount":"000123456789012345678901234567890.123456789012345678900","instant":"2024-03-31T03:00:00+02:00","document":EXACT_JSON}),
            },
        );
        let corrected = runtime
            .invoke(&actor("admin"), action.clone())
            .await
            .unwrap();
        assert_eq!(corrected.revision, 4);
        let final_row = store.load(&key("a")).unwrap().unwrap();
        assert_eq!(
            final_row.value.as_ref().unwrap()["amount"],
            "123456789012345678901234567890.1234567890123456789"
        );
        assert_eq!(
            final_row.value.as_ref().unwrap()["instant"],
            "2024-03-31T01:00:00.000000000Z"
        );
        assert_eq!(
            final_row.value.as_ref().unwrap()["amounts"],
            json!({"exact":"0.0123","empty":null})
        );
        rejected(
            &runtime,
            store.as_ref(),
            invoke(
                "a",
                Some(4),
                "invalid-action",
                Operation::Action {
                    name: "correct".into(),
                    input: json!({"amount":1.2,"instant":"2024-03-31T01:00:00Z","document":"{}"}),
                },
            ),
        )
        .await;
        let projected = runtime
            .read_projected(&actor("alice"), Measurement::KIND, "a")
            .await
            .unwrap();
        assert!(!projected.value.unwrap().contains_key("secret"));
        assert!(matches!(
            runtime.read::<Measurement>(&actor("alice"), "a").await,
            Err(Error::Denied)
        ));
        assert!(
            runtime
                .query_spec_projected(
                    &actor("alice"),
                    Measurement::KIND,
                    QuerySpec::equal("secret", json!("HIDDEN-SEMANTIC-STORY"))
                )
                .await
                .is_err()
        );
        let counts = store.counts();
        assert_eq!(counts, [3, 6, 6, 0]);
        for request in [
            invoke(
                "a",
                Some(4),
                "private-write",
                patch(&[("secret", FieldUpdate::Set(json!("changed")))]),
            ),
            action.clone(),
        ] {
            let subject = if matches!(request.operation, Operation::Action { .. }) {
                "mallory"
            } else {
                "alice"
            };
            assert!(matches!(
                runtime.invoke(&actor(subject), request).await,
                Err(Error::Denied)
            ));
        }
        assert_eq!(store.load(&key("a")).unwrap(), Some(final_row.clone()));
        assert_eq!(store.counts(), counts);
        assert!(
            runtime
                .query_spec_projected(&actor("mallory"), Measurement::KIND, QuerySpec::all())
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            runtime
                .journal(&actor("mallory"), Measurement::KIND, None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        assert!(
            !serde_json::to_string(
                &runtime
                    .journal(&actor("alice"), Measurement::KIND, None)
                    .await
                    .unwrap()
            )
            .unwrap()
            .contains("HIDDEN-SEMANTIC-STORY")
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(store);

        let store = fixture.open();
        let runtime = self::runtime(store.clone());
        assert_eq!(store.load(&key("a")).unwrap(), Some(final_row));
        assert_eq!(store.counts(), counts);
        assert_eq!(
            runtime.invoke(&actor("admin"), action).await.unwrap(),
            corrected
        );
        assert_eq!(store.counts(), counts);
        let restored = runtime
            .read::<Measurement>(&actor("admin"), "a")
            .await
            .unwrap()
            .value
            .unwrap();
        assert_eq!(restored.optional, Presence::Missing);
        assert_eq!(restored.document.as_str(), EXACT_JSON);
        assert_eq!(restored.color.as_str(), "#aabbcc");
        assert_eq!(restored.day.as_str(), "2000-02-29");
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(store);
    }
}
