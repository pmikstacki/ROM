#[tokio::test]
async fn sqlite_runs_real_compensation_and_counterexamples() {
    let rows = rom_compensation_probe::run_backend("sqlite").await;
    assert!(
        rows.len() >= 18,
        "all requested behaviors must have persisted evidence"
    );
    assert!(rows.iter().all(|r| r["passed"] == true));
}
#[tokio::test]
async fn redb_runs_same_domain_and_recovery_contract() {
    let rows = rom_compensation_probe::run_backend("redb").await;
    assert!(rows.len() >= 18, "both storage adapters run the same cases");
    assert!(rows.iter().all(|r| r["passed"] == true));
}
#[tokio::test]
async fn sqlite_recovers_after_real_child_process_post_commit_exit() {
    let cases = rom_compensation_probe::run_crash_cases(std::path::Path::new(env!(
        "CARGO_BIN_EXE_rom-compensation-probe"
    )))
    .await;
    assert_eq!(cases.len(), 2);
    assert!(
        cases
            .iter()
            .all(|c| c["passed"] == true && c["checks"]["release_entries"] == 1)
    );
}
