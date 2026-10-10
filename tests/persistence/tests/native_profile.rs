#[test]
fn sqlite_native_engine_matches_selected_build_profile() {
    let linked = rusqlite::version();
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let source_id: String = connection
        .query_row("SELECT sqlite_source_id()", [], |row| row.get(0))
        .unwrap();
    let mut statement = connection.prepare("PRAGMA compile_options").unwrap();
    let mut options: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    options.sort();
    println!("Rust-linked SQLite engine: {linked}");
    println!("Rust-linked SQLite source ID: {source_id}");
    println!("Rust-linked SQLite compile options: {}", options.join(","));
    if let Ok(expected) = std::env::var("ROM_EXPECT_SQLITE_VERSION") {
        assert_eq!(
            linked, expected,
            "native build silently linked a different engine"
        );
        let expected_source = std::env::var("ROM_EXPECT_SQLITE_SOURCE_ID")
            .expect("selected native profile must declare its exact source ID");
        assert_eq!(
            source_id, expected_source,
            "native SQLite source ID differs"
        );
        for required in ["THREADSAFE=1", "ENABLE_COLUMN_METADATA"] {
            assert!(
                options.iter().any(|option| option == required),
                "native SQLite profile lacks required option {required}"
            );
        }
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
