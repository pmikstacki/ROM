use super::*;

#[test]
fn linked_engine_queries_actual_in_memory_connection_without_a_persisted_path() {
    let identity = Sqlite::linked_engine_identity().unwrap();
    identity.validate().unwrap();
    assert_eq!(identity.version, rusqlite::version());
    assert!(!identity.source_id.is_empty());
    assert!(!identity.compile_options.is_empty());
}

#[test]
fn linked_engine_rejects_empty_nonprintable_overlong_and_duplicate_metadata() {
    let valid = || LinkedEngineIdentity {
        version: "3.53.4".into(),
        source_id: "2026-07-24 example-source".into(),
        compile_options: vec!["THREADSAFE=1".into()],
    };
    valid().validate().unwrap();
    for case in 0..7 {
        let mut identity = valid();
        match case {
            0 => identity.version.clear(),
            1 => identity.version = "v".repeat(33),
            2 => identity.source_id = "s".repeat(193),
            3 => identity.compile_options = vec!["x".repeat(257)],
            4 => identity.compile_options = vec!["x".into(); 129],
            5 => identity.compile_options = vec!["x".into(), "x".into()],
            _ => identity.compile_options = vec!["line\nfeed".into()],
        }
        assert_eq!(identity.validate(), Err(rom::Error::Storage));
    }
}
