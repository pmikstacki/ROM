use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Debug, Resource)]
#[resource(name = "query-items")]
struct Item {
    amount: u64,
    title: String,
    score: FiniteF64,
    note: Presence<Option<String>>,
    visible: bool,
}
const SET: Action<Item, u64> = Action::new("set", |r, n| {
    r.amount = n;
    Ok(vec![])
});
fn actor() -> Actor {
    Actor::trusted("tests", "owner")
}
struct Fixture {
    runtime: Runtime,
    path: std::path::PathBuf,
}
impl Fixture {
    fn new(redb: bool, restricted: bool) -> Self {
        let definition = Item::definition()
            .policy(|a, access, r| {
                matches!(access, Access::Write) || r.visible || a.subject == "owner"
            })
            .allow_all_fields()
            .action(SET);
        let definition = if restricted {
            definition
                .sort_policy(|a, _| a.subject != "no-sort")
                .field_policy(|a, access, field, _| {
                    matches!(access, Access::Write)
                        || a.subject != "hidden-field"
                        || field != "amount"
                })
        } else {
            definition
        };
        Self::configured(redb, definition, Limits::default())
    }
    fn configured(redb: bool, definition: Definition<Item>, limits: Limits) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-query-plan-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let runtime = Runtime::builder()
            .limits(limits)
            .resource(definition)
            .build(storage, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self { runtime, path }
    }
    async fn add(&self, id: &str, amount: u64, note: Presence<Option<String>>, visible: bool) {
        self.runtime
            .execute(
                &actor(),
                Command::create(
                    id,
                    Item {
                        amount,
                        title: "é".into(),
                        score: FiniteF64::new(amount as f64).unwrap(),
                        note,
                        visible,
                    },
                )
                .idempotency(&format!("create-{id}")),
            )
            .await
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
#[tokio::test]
async fn shared_exact_ranges_multisort_typed_wire_live_and_moving_anchor() {
    for redb in [false, true] {
        let f = Fixture::new(redb, false);
        f.add("a", u64::MAX - 1, Presence::Missing, true).await;
        f.add("b", u64::MAX, Presence::Value(None), true).await;
        f.add("c", u64::MAX, Presence::Value(Some("z".into())), true)
            .await;
        let query = Item::amount_field()
            .at_least(u64::MAX - 1)
            .order_by(Item::amount_field(), Direction::Asc)
            .order_by(Item::note_field(), Direction::Asc)
            .limit(1);
        let first = f.runtime.query(&actor(), &query).await.unwrap();
        assert_eq!(first[0].id, "a");
        let wire: QuerySpec =
            serde_json::from_value(serde_json::to_value(query.spec()).unwrap()).unwrap();
        assert_eq!(
            f.runtime
                .query_spec_projected(&actor(), Item::KIND, wire.clone())
                .await
                .unwrap()[0]
                .key
                .id,
            "a"
        );
        let mut live = f.runtime.live(&actor(), query.clone()).await.unwrap();
        assert_eq!(live.changed().await.unwrap()[0].id, "a");
        let next = query.clone().after_snapshot(&first[0]).unwrap().limit(9);
        assert_eq!(
            f.runtime
                .query(&actor(), &next)
                .await
                .unwrap()
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            ["b", "c"]
        );
        f.runtime
            .execute(
                &actor(),
                Command::action("a", SET, u64::MAX)
                    .at_revision(first[0].revision)
                    .idempotency("move-a"),
            )
            .await
            .unwrap();
        assert_eq!(
            f.runtime
                .query(&actor(), &next)
                .await
                .unwrap()
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        f.runtime
            .execute(
                &actor(),
                Command::action("a", SET, 0)
                    .at_revision(2)
                    .idempotency("move-before"),
            )
            .await
            .unwrap();
        assert_eq!(live.changed().await.unwrap()[0].id, "b");
        let mut projected_live = f
            .runtime
            .live_spec_projected(&actor(), Item::KIND, next.spec().clone())
            .await
            .unwrap();
        assert_eq!(projected_live.changed().await.unwrap()[0].key.id, "b");
        f.runtime
            .execute(
                &actor(),
                Command::<Item>::delete("a")
                    .at_revision(3)
                    .idempotency("delete-anchor"),
            )
            .await
            .unwrap();
        f.add("0", u64::MAX - 1, Presence::Missing, true).await;
        assert_eq!(ids(&f, next.spec().clone()).await, ["b", "c"]);
        assert!(
            f.runtime
                .query(&actor(), &query.clone().after_id("a"))
                .await
                .is_err()
        );
        assert!(
            query
                .clone()
                .and_where(Item::amount_field().equals(1).limit(1))
                .is_err()
        );
        let mut wrong = next.spec().clone();
        wrong.after.as_mut().unwrap().schema_version += 1;
        assert!(
            f.runtime
                .query_spec_projected(&actor(), Item::KIND, wrong)
                .await
                .is_err()
        );
    }
}
#[tokio::test]
async fn shared_sort_authority_is_checked_on_empty_and_hidden_values() {
    for redb in [false, true] {
        let f = Fixture::new(redb, true);
        let q = Query::<Item>::all().order_by(Item::amount_field(), Direction::Asc);
        let no_sort = Actor::trusted("tests", "no-sort");
        assert_eq!(
            f.runtime.query(&no_sort, &q).await.unwrap_err(),
            Error::Denied
        );
        f.add("hidden", 1, Presence::Missing, false).await;
        let hidden_field = Actor::trusted("tests", "hidden-field");
        assert!(
            f.runtime
                .query_spec_projected(&hidden_field, Item::KIND, q.spec().clone())
                .await
                .unwrap()
                .is_empty()
        );
        f.add("visible", 2, Presence::Missing, true).await;
        assert_eq!(
            f.runtime
                .query_spec_projected(&hidden_field, Item::KIND, q.spec().clone())
                .await
                .unwrap_err(),
            Error::Denied
        );
    }
}

async fn ids(f: &Fixture, spec: QuerySpec) -> Vec<String> {
    f.runtime
        .query_spec_projected(&actor(), Item::KIND, spec)
        .await
        .unwrap()
        .into_iter()
        .map(|v| v.key.id)
        .collect()
}
#[tokio::test]
async fn shared_null_missing_ranges_and_both_sort_directions() {
    for redb in [false, true] {
        let f = Fixture::new(redb, false);
        f.add("missing", 0, Presence::Missing, true).await;
        f.add("null", 1, Presence::Value(None), true).await;
        f.add("value", 2, Presence::Value(Some("x".into())), true)
            .await;
        assert_eq!(
            ids(&f, QuerySpec::all().order_by("note", Direction::Asc)).await,
            ["missing", "null", "value"]
        );
        assert_eq!(
            ids(&f, QuerySpec::all().order_by("note", Direction::Desc)).await,
            ["value", "null", "missing"]
        );
        assert_eq!(
            ids(
                &f,
                QuerySpec::all().compare("note", CompareOp::Ge, json!(""))
            )
            .await,
            ["value"]
        );
        assert_eq!(
            ids(
                &f,
                QuerySpec::all().compare("note", CompareOp::Ne, json!("x"))
            )
            .await,
            ["null"]
        );
        assert_eq!(
            ids(&f, QuerySpec::equal("note", Value::Null)).await,
            ["null"]
        );
        assert_eq!(ids(&f, QuerySpec::absent("note")).await, ["missing"]);
        assert!(
            f.runtime
                .query(&actor(), &Item::note_field().less_than(Presence::Missing))
                .await
                .is_err()
        );
        let q = Item::note_field().not_equals(Presence::Missing);
        assert_eq!(ids(&f, q.spec().clone()).await, ["null", "value"]);
    }
}
#[tokio::test]
async fn shared_anchor_binding_shape_bounds_and_dynamic_roundtrip() {
    for redb in [false, true] {
        let f = Fixture::new(redb, false);
        f.add("a", 1, Presence::Missing, true).await;
        let spec = QuerySpec::all()
            .compare("amount", CompareOp::Ge, json!(0))
            .order_by("amount", Direction::Asc);
        let view = f
            .runtime
            .query_spec_projected(&actor(), Item::KIND, spec.clone())
            .await
            .unwrap()
            .remove(0);
        let anchor = f
            .runtime
            .query_anchor(&actor(), &spec, &view)
            .await
            .unwrap();
        assert!(
            ids(&f, spec.clone().after(anchor.clone()).limit(2))
                .await
                .is_empty()
        );
        let mut bads = vec![];
        let mut a = anchor.clone();
        a.kind = "other".into();
        bads.push(a);
        let mut a = anchor.clone();
        a.version = 2;
        bads.push(a);
        let mut a = anchor.clone();
        a.schema_version += 1;
        bads.push(a);
        let mut a = anchor.clone();
        a.comparisons[0].value = json!(2);
        bads.push(a);
        let mut a = anchor.clone();
        a.order[0].direction = Direction::Desc;
        bads.push(a);
        let mut a = anchor.clone();
        a.values.clear();
        bads.push(a);
        let mut a = anchor.clone();
        a.values[0] = AnchorValue::Value(json!("wrong"));
        bads.push(a);
        let mut a = anchor.clone();
        a.values[0] = AnchorValue::Missing;
        bads.push(a);
        let mut a = anchor.clone();
        a.id.clear();
        bads.push(a);
        let empty = Fixture::new(redb, false);
        for a in bads {
            assert!(
                empty
                    .runtime
                    .query_spec_projected(&actor(), Item::KIND, spec.clone().after(a))
                    .await
                    .is_err()
            );
        }
        assert!(
            empty
                .runtime
                .query_spec_projected(
                    &actor(),
                    Item::KIND,
                    spec.clone().order_by("amount", Direction::Desc)
                )
                .await
                .is_err()
        );
        let mut arbitrary = anchor.clone();
        arbitrary.values[0] = AnchorValue::Value(json!(0));
        arbitrary.id = "chosen".into();
        assert_eq!(ids(&f, spec.clone().after(arbitrary)).await, ["a"]);
        let mut huge = anchor;
        huge.id = "x".repeat(20_000);
        assert_eq!(
            empty
                .runtime
                .query_spec_projected(&actor(), Item::KIND, spec.after(huge))
                .await
                .unwrap_err(),
            Error::TooLarge
        );
        let malformed = json!({"after":{"version":1,"kind":"query-items","schema_version":1,"filters":[],"comparisons":[],"order":[],"id":"a","values":[],"secret":true}});
        assert!(serde_json::from_value::<QuerySpec>(malformed).is_err());
    }
}
#[derive(Clone, Debug)]
struct Trimmed(String);
impl Field for Trimmed {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(v: Value) -> Result<Self> {
        v.as_str()
            .map(|v| Self(v.trim().into()))
            .ok_or_else(|| Error::invalid("codec", "text"))
    }
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "query-codecs")]
struct Codecs {
    score: FiniteF64,
    signed: i64,
    text: Trimmed,
    list: Vec<u64>,
}
#[tokio::test]
async fn shared_float_custom_codec_signed_extremes_and_structural_equality() {
    for redb in [false, true] {
        let base = Fixture::new(redb, false);
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(base.path.with_extension("codec")).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(base.path.with_extension("codec")).unwrap())
        };
        let runtime = Runtime::builder()
            .resource(
                Codecs::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true),
            )
            .build(storage, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        for (id, score, signed, text) in [
            ("a", -0.0, i64::MIN, " z "),
            ("b", 0.0, i64::MAX, "a"),
            ("c", 1.5, 0, "é"),
        ] {
            runtime
                .execute(
                    &actor(),
                    Command::create(
                        id,
                        Codecs {
                            score: FiniteF64::new(score).unwrap(),
                            signed,
                            text: Trimmed(text.into()),
                            list: vec![1, 2],
                        },
                    )
                    .idempotency(id),
                )
                .await
                .unwrap();
        }
        let q = Codecs::score_field()
            .equals(FiniteF64::new(0.0).unwrap())
            .order_by(Codecs::signed_field(), Direction::Desc);
        assert_eq!(
            runtime
                .query(&actor(), &q)
                .await
                .unwrap()
                .iter()
                .map(|v| v.id.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
        let q = Codecs::text_field()
            .at_least(Trimmed(" z ".into()))
            .and_where(Codecs::list_field().equals(vec![1, 2]))
            .unwrap()
            .order_by(Codecs::text_field(), Direction::Asc);
        let rows = runtime.query(&actor(), &q).await.unwrap();
        assert_eq!(
            rows.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            ["a", "c"]
        );
        let after = q.after_snapshot(&rows[0]).unwrap();
        assert_eq!(runtime.query(&actor(), &after).await.unwrap()[0].id, "c");
        let mut bad = after.spec().clone();
        bad.after.as_mut().unwrap().values[0] = AnchorValue::Value(json!(" z "));
        assert!(
            runtime
                .query_spec_projected(&actor(), Codecs::KIND, bad)
                .await
                .is_err()
        );
        let float_wire = QuerySpec::all()
            .compare("score", CompareOp::Ge, json!(0))
            .order_by("score", Direction::Asc);
        assert_eq!(
            runtime
                .query_spec_projected(&actor(), Codecs::KIND, float_wire)
                .await
                .unwrap()
                .len(),
            3
        );
        assert!(
            runtime
                .query(&actor(), &Codecs::list_field().less_than(vec![3]))
                .await
                .is_err()
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        std::fs::remove_file(base.path.with_extension("codec")).unwrap();
    }
}

#[tokio::test]
async fn shared_default_sort_deny_and_small_page_does_not_bypass_snapshot_bound() {
    for redb in [false, true] {
        let def = Item::definition()
            .policy(|_, _, _| true)
            .field_policy(|_, _, _, _| true)
            .query_policy(|_, _| true);
        let f = Fixture::configured(redb, def, Limits::default());
        let query = Query::<Item>::all().order_by(Item::amount_field(), Direction::Asc);
        assert_eq!(
            f.runtime.query(&actor(), &query).await.unwrap_err(),
            Error::Denied
        );
        let def = Item::definition().policy(|_, _, _| true).allow_all_fields();
        let f = Fixture::configured(
            redb,
            def,
            Limits {
                snapshot_rows: 1,
                ..Limits::default()
            },
        );
        f.add("a", 1, Presence::Missing, true).await;
        f.add("b", 2, Presence::Missing, true).await;
        assert_eq!(
            f.runtime
                .query(&actor(), &query.limit(1))
                .await
                .unwrap_err(),
            Error::TooLarge
        );
        let mut too_many = QuerySpec::all();
        for _ in 0..33 {
            too_many = too_many.and("amount", json!(1));
        }
        assert_eq!(
            f.runtime
                .query_spec_projected(&actor(), Item::KIND, too_many)
                .await
                .unwrap_err(),
            Error::TooLarge
        );
    }
}
