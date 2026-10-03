#[tokio::test]
async fn same_author_application_runs_on_sqlite_and_redb() {
    rom_demo::smoke::run(false).await.unwrap();
    rom_demo::smoke::run(true).await.unwrap();
}
#[test]
fn custom_field_canonicalizes_and_rejects_invalid_stock_codes() {
    use rom::{Field, json};
    use rom_demo::StockCode;
    assert_eq!(
        StockCode::decode(json!("  bin-7 ")).unwrap().encode(),
        json!("BIN-7")
    );
    assert!(StockCode::decode(json!("bad/code")).is_err());
    assert!(StockCode::decode(json!(12)).is_err());
}

#[tokio::test]
async fn local_cli_discovers_only_deliberately_published_domain_metadata() {
    use std::sync::{Arc, Mutex};
    let runtime = rom_demo::declarations(Arc::new(Mutex::new(Vec::new())))
        .unwrap()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            rom::Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let discovery = runtime.discover(&rom_demo::session_actor()).await.unwrap();
    assert_eq!(
        discovery
            .resources
            .iter()
            .map(|resource| resource.kind.as_str())
            .collect::<Vec<_>>(),
        ["checkouts", "inventory", "reservation-stock", "tasks"]
    );
    assert_eq!(
        discovery.resources[1]
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["code", "quantity"]
    );
    assert_eq!(
        discovery.resources[3]
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["done", "title"]
    );
    assert_eq!(discovery.resources[3].actions, ["complete"]);
    assert_eq!(
        discovery.resources[0]
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["payment_outcome", "reservation_id", "stock_id"]
    );
    assert_eq!(discovery.resources[0].actions, ["record-payment"]);
    assert_eq!(
        discovery.resources[2]
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["reservations", "total"]
    );
    assert_eq!(discovery.resources[2].actions, ["release", "reserve"]);
    assert!(matches!(
        runtime
            .discover(&rom::Actor::trusted("demo-host", "unprovisioned"))
            .await,
        Err(rom::Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}
