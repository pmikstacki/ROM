#[test]
fn separate_processes_reopen_and_replay_one_durable_effect() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("PROTOTYPE-process-restart.sqlite");
    for _ in 0..2 {
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_rom-dedup-cache-prototype"))
            .arg("persist-demo")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let db = rusqlite::Connection::open(&path).unwrap();
        for sql in [
            "SELECT count(*) FROM receipts",
            "SELECT count(*) FROM events",
            "SELECT sum(value) FROM resources",
            "SELECT max(revision) FROM resources",
        ] {
            assert_eq!(db.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        }
    }
}
