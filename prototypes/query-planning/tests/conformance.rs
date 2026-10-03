use rom_query_planning_prototype::*;
fn row(id: &str, amount: u64, title: &str, visible: bool) -> Row {
    Row {
        id: id.into(),
        amount,
        title: title.into(),
        note: Note::Missing,
        visible,
    }
}
#[test]
fn strategies_compile_to_identical_canonical_plans() {
    let a = generated::InventoryQuery::new()
        .amount_ge(9)
        .title_equals(" value ")
        .order_amount_desc()
        .order_title_asc()
        .finish();
    let b = hybrid::Builder::new()
        .filter(hybrid::AMOUNT.ge(9))
        .filter(hybrid::TITLE.eq(" value "))
        .order(hybrid::AMOUNT.desc())
        .order(hybrid::TITLE.asc())
        .finish();
    let c=compile_wire("inventory",&serde_json::json!({"filters":[{"field":"amount","op":"ge","value":9},{"field":"title","op":"eq","value":" value "}],"order":[{"field":"amount","descending":true},{"field":"title","descending":false}]})).unwrap();
    assert_eq!(a, b);
    assert_eq!(a, c);
    assert_eq!(a.filters[1], Predicate::TitleEq("value".into()));
    let ticket = generated::TicketQuery::new()
        .priority_ge(9)
        .title_equals("value")
        .order_priority_desc()
        .order_title_asc()
        .finish();
    assert_eq!(ticket, a);
    assert!(
        compile_wire(
            "tickets",
            &serde_json::json!({"filters":[{"field":"amount","op":"ge","value":9}],"order":[]})
        )
        .is_err()
    );
}
#[test]
fn sqlite_exact_unsigned_missing_null_unicode_and_ties_match_oracle() {
    let mut rows = vec![
        row("a", u64::MAX, "é", true),
        row("b", i64::MAX as u64 + 1, "a\0b", true),
        row("c", (1 << 53) + 1, "e\u{301}", true),
        row("d", 1 << 53, "Z", true),
        row("e", u64::MAX, "é", true),
    ];
    rows[1].note = Note::Null;
    rows[2].note = Note::Value("".into());
    let db = Sqlite::new(&rows, true).unwrap();
    for plan in [
        Plan {
            order: vec![Order(Field::Amount, true), Order(Field::Title, false)],
            ..Plan::default()
        },
        Plan {
            order: vec![Order(Field::Title, false)],
            ..Plan::default()
        },
        Plan {
            order: vec![Order(Field::Note, false)],
            ..Plan::default()
        },
        Plan {
            filters: vec![Predicate::NoteEq(Note::Null)],
            ..Plan::default()
        },
        Plan {
            filters: vec![Predicate::AmountGe((1 << 53) + 1)],
            ..Plan::default()
        },
    ] {
        assert_eq!(
            db.query(&plan, None, 99, 100).unwrap().ids(),
            oracle(&rows, &plan, None, 99).ids()
        );
    }
    assert_eq!(
        db.query(
            &Plan {
                filters: vec![Predicate::NoteEq(Note::Null)],
                ..Plan::default()
            },
            None,
            99,
            100
        )
        .unwrap()
        .ids(),
        ["b"]
    );
}
#[test]
fn negative_controls_expose_lossy_numeric_and_null_translation() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    let equal: bool = db
        .query_row(
            "SELECT CAST(? AS REAL)=CAST(? AS REAL)",
            [u64::MAX.to_string(), (u64::MAX - 1).to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        equal,
        "lossy negative control no longer collapses adjacent integers"
    );
    let conflated: bool = db
        .query_row(
            "SELECT json_extract('{}','$.note') IS json_extract('{\"note\":null}','$.note')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(conflated);
}
#[test]
fn authorization_precedes_result_limit_and_budget_exhaustion_is_explicit() {
    let rows = vec![
        row("a", 1, "a", false),
        row("b", 2, "b", false),
        row("c", 3, "c", true),
    ];
    let db = Sqlite::new(&rows, true).unwrap();
    let plan = Plan::default();
    assert!(db.query(&plan, None, 0, 3).unwrap().rows.is_empty());
    assert_eq!(db.query(&plan, None, 1, 3).unwrap().ids(), ["c"]);
    assert!(db.query(&plan, None, 1, 2).is_err());
    assert!(db.wrong_limit_before_policy(&plan, 1).unwrap().is_empty());
}
#[test]
fn moving_anchor_uses_observed_values_and_id_tiebreaker() {
    let plan = Plan {
        order: vec![Order(Field::Amount, false)],
        ..Plan::default()
    };
    let mut rows = vec![
        row("a", 1, "a", true),
        row("b", 1, "b", true),
        row("c", 2, "c", true),
    ];
    let first = oracle(&rows, &plan, None, 1);
    assert_eq!(first.ids(), ["a"]);
    let anchor = first.rows[0].clone();
    rows[0].amount = 3;
    rows.push(row("before", 0, "before", true));
    let next = Sqlite::new(&rows, true)
        .unwrap()
        .query(&plan, Some(&anchor), 99, 99)
        .unwrap();
    assert_eq!(next.ids(), ["b", "c", "a"]);
    rows.retain(|r| r.id != "a");
    assert_eq!(oracle(&rows, &plan, Some(&anchor), 99).ids(), ["b", "c"]);
}
#[test]
fn sort_permission_is_checked_even_for_empty_data() {
    assert!(
        authorize(
            &Plan {
                order: vec![Order(Field::Amount, false)],
                ..Plan::default()
            },
            false
        )
        .is_err()
    );
}

#[test]
fn redb_persistent_scan_has_same_exact_results_and_bounds() {
    let root = std::env::var("QUERY_PROBE_SCRATCH")
        .unwrap_or_else(|_| "/var/tmp/rom-query-probe-data".into());
    std::fs::create_dir_all(&root).unwrap();
    let path = std::path::Path::new(&root).join("conformance.redb");
    let mut rows = dataset(100);
    let plan = Plan {
        order: vec![Order(Field::Amount, true), Order(Field::Note, false)],
        ..Plan::default()
    };
    {
        let db = Redb::new(&path, &rows).unwrap();
        assert_eq!(
            db.query(&plan, None, 20, 100).unwrap().rows,
            oracle(&rows, &plan, None, 20).rows
        );
        assert!(db.query(&plan, None, 20, 99).is_err());
        db.mutate_amount(&rows[0].id, u64::MAX).unwrap();
        rows[0].amount = u64::MAX;
    }
    let reopened = Redb::reopen(&path).unwrap();
    assert_eq!(
        reopened.query(&plan, None, 20, 100).unwrap().rows,
        oracle(&rows, &plan, None, 20).rows
    );
    assert_eq!(
        reopened.query(&plan, None, 20, 100).unwrap().ids()[0],
        "00000000"
    );
}

#[test]
fn synthetic_transactional_journal_is_invariant_across_query_strategies_and_reopen() {
    let root = std::env::var("QUERY_PROBE_SCRATCH")
        .unwrap_or_else(|_| "/var/tmp/rom-query-probe-data".into());
    std::fs::create_dir_all(&root).unwrap();
    let mut expected_events = None;
    for strategy in ["snapshot", "unindexed", "indexed"] {
        let path = std::path::Path::new(&root).join(format!("journal-{strategy}.sqlite"));
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        let mut rows = dataset(30);
        let mut db = Sqlite::create_at(&path, &rows, strategy != "unindexed").unwrap();
        let plan = Plan {
            order: vec![Order(Field::Amount, true), Order(Field::Title, false)],
            ..Plan::default()
        };
        for (index, amount) in [(0, u64::MAX), (10, 999), (20, 0), (0, 17)] {
            db.mutate_amount(&rows[index].id, amount).unwrap();
            rows[index].amount = amount;
            let got = if strategy == "snapshot" {
                let all = db.snapshot(100).unwrap();
                oracle(&all.rows, &plan, None, 10)
            } else {
                db.query(&plan, None, 10, 100).unwrap()
            };
            assert_eq!(got.rows, oracle(&rows, &plan, None, 10).rows);
        }
        let events = db.journal().unwrap();
        assert_eq!(events.len(), 4);
        if let Some(expected) = &expected_events {
            assert_eq!(&events, expected);
        } else {
            expected_events = Some(events.clone());
        }
        drop(db);
        let reopened = Sqlite::reopen(&path).unwrap();
        assert_eq!(reopened.journal().unwrap(), events);
        assert_eq!(
            reopened.query(&plan, None, 10, 100).unwrap().rows,
            oracle(&rows, &plan, None, 10).rows
        );
    }
}
