use rom::{
    Actor, Command, Error, Field, FiniteF64, QuerySpec, Resource, Runtime, Shape, Value, json,
};
use rom_sqlite::Sqlite;
use std::sync::Arc;
#[derive(Clone)]
struct Trimmed(String);
impl Field for Trimmed {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(v: Value) -> rom::Result<Self> {
        v.as_str()
            .map(|s| Self(s.trim().into()))
            .ok_or(Error::Denied)
    }
}
#[derive(Clone, Resource)]
#[resource(name = "items")]
struct Item {
    title: Trimmed,
    price: FiniteF64,
    done: bool,
}
async fn fixture() -> Runtime {
    let runtime = Runtime::builder()
        .resource(Item::definition().policy(|_, _, _| true).allow_all_fields())
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    for (id, done) in [("c", false), ("a", false), ("b", true)] {
        runtime
            .execute(
                &actor(),
                Command::create(
                    id,
                    Item {
                        title: Trimmed("same".into()),
                        price: FiniteF64::new(0.0).unwrap(),
                        done,
                    },
                )
                .idempotency(id),
            )
            .await
            .unwrap();
    }
    runtime
}
fn actor() -> Actor {
    Actor::trusted("host", "alice")
}
#[tokio::test]
async fn projected_and_typed_predicates_use_the_actual_field_codec() {
    let runtime = fixture().await;
    assert_eq!(
        runtime
            .query_projected(&actor(), "items", "price", json!(0))
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        runtime
            .query_projected(&actor(), "items", "title", json!(" same "))
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        runtime
            .query(
                &actor(),
                &Item::title_field().equals(Trimmed(" same ".into()))
            )
            .await
            .unwrap()
            .len(),
        3
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn conjunction_and_keyset_pages_share_authorized_identity_order() {
    let runtime = fixture().await;
    let first = Item::title_field()
        .equals(Trimmed("same".into()))
        .and(Item::done_field(), false)
        .limit(1);
    let rows = runtime.query(&actor(), &first).await.unwrap();
    assert_eq!(
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["a"]
    );
    let second = first.after_id("a");
    assert_eq!(runtime.query(&actor(), &second).await.unwrap()[0].id, "c");
    let projected = runtime
        .query_spec_projected(&actor(), "items", second.spec().clone())
        .await
        .unwrap();
    assert_eq!(projected[0].key.id, "c");
    let mut live = runtime
        .live_spec_projected(&actor(), "items", second.spec().clone())
        .await
        .unwrap();
    assert_eq!(live.changed().await.unwrap()[0].key.id, "c");
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn query_bounds_and_unknown_operators_fail_explicitly() {
    let runtime = fixture().await;
    assert!(matches!(
        runtime
            .query_spec_projected(&actor(), "items", QuerySpec::all().limit(0))
            .await,
        Err(Error::TooLarge)
    ));
    assert!(
        runtime
            .query_spec_projected(&actor(), "items", QuerySpec::equal("unknown", json!(true)))
            .await
            .is_err()
    );
    assert!(serde_json::from_value::<QuerySpec>(json!({"or":[]})).is_err());
    let huge = QuerySpec::equal("title", json!("x".repeat(20_000)));
    assert!(matches!(
        runtime.query_spec_projected(&actor(), "items", huge).await,
        Err(Error::TooLarge)
    ));
    runtime.shutdown().await.unwrap();
}
