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
