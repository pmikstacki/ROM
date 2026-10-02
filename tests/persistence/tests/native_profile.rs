#[test]
fn sqlite_native_engine_matches_selected_build_profile() {
    let linked = rusqlite::version();
    println!("Rust-linked SQLite engine: {linked}");
    if let Ok(expected) = std::env::var("ROM_EXPECT_SQLITE_VERSION") {
        assert_eq!(
            linked, expected,
            "native build silently linked a different engine"
        );
    } else {
        assert_eq!(
            linked, "3.53.2",
            "review the changed default native dependency"
        );
    }
}
