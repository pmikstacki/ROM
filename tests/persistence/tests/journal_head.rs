use rom::{Bundle, Error, JournalCursor, Key, Receipt, Row, Storage, StorageLimits, json};
fn open(redb: bool, path: &std::path::Path) -> Box<dyn Storage> {
    let limits = StorageLimits {
        journal_rows: 1,
        ..Default::default()
    };
    if redb {
        Box::new(rom_redb::Redb::open_with_limits(path, limits).unwrap())
    } else {
        Box::new(rom_sqlite::Sqlite::open_with_limits(path, limits).unwrap())
    }
}
#[test]
fn explicit_head_survives_reopen_and_allows_recovery_after_retention_gap() {
    for redb in [false, true] {
        let path =
            std::env::temp_dir().join(format!("rom-journal-head-{}-{redb}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = open(redb, &path);
        let beginning = db.journal_head("things").unwrap();
        assert_eq!(beginning.position, 0);
        for i in 1..=2 {
            db.commit(&Bundle {
                expected: None,
                changed: true,
                effects: vec![],
                reactions: vec![],
                reaction_limits: None,
                receipt: Receipt {
                    identity: i.to_string(),
                    fingerprint: i.to_string(),
                    row: Row {
                        key: Key {
                            kind: "things".into(),
                            id: i.to_string(),
                        },
                        revision: 1,
                        value: Some(json!({"value":i})),
                    },
                },
            })
            .unwrap();
        }
        assert_eq!(
            db.journal("things", Some(&beginning), 4, 4096),
            Err(Error::HistoryGap)
        );
        let head = db.journal_head("things").unwrap();
        assert_eq!(head.position, 2);
        assert_eq!(head.generation, beginning.generation);
        assert_eq!(
            db.journal_head("other").unwrap(),
            JournalCursor {
                kind: "other".into(),
                ..head.clone()
            }
        );
        drop(db);
        let db = open(redb, &path);
        assert_eq!(db.journal_head("things").unwrap(), head);
        assert!(
            db.journal("things", Some(&head), 4, 4096)
                .unwrap()
                .events
                .is_empty()
        );
        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
