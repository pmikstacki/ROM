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

#[test]
fn native_extension_manifest_matches_compiled_contract_and_fresh_archive() {
    let profile: serde_json::Value =
        serde_json::from_str(include_str!("../../../extensions/native-alpha-v1.json")).unwrap();
    assert_eq!(profile["profile_version"], rom_conformance::PROFILE_VERSION);
    assert_eq!(profile["package_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(profile["rust_version"], env!("CARGO_PKG_RUST_VERSION"));
    assert_eq!(
        profile["query_versions"]["semantics"],
        rom::QUERY_SEMANTICS_VERSION
    );
    assert_eq!(
        profile["query_versions"]["profile"],
        rom::QUERY_PROFILE_VERSION
    );
    assert_eq!(
        profile["query_versions"]["encoding"],
        rom::QUERY_ENCODING_VERSION
    );
    assert_eq!(
        profile["operator_protocol_version"],
        rom::operator::OPERATOR_PROTOCOL_VERSION
    );
    assert_eq!(profile["native_storage_format"], rom_backup::STORAGE_FORMAT);

    let path = std::env::temp_dir().join(format!(
        "rom-extension-profile-{}.archive",
        std::process::id()
    ));
    // backup_to creates the destination exclusively; a preoccupied path must fail.
    let result = rom_sqlite::Sqlite::open(":memory:")
        .unwrap()
        .backup_to(&path, Default::default())
        .unwrap();
    let removal = std::fs::remove_file(path);
    assert_eq!(profile["archive_format"], result.archive_version);
    assert_eq!(profile["native_storage_format"], result.storage_format);
    removal.unwrap();
}
