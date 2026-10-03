use super::{encoding::encode, query::query_read};
use rom::*;
use rusqlite::{Connection, params};
fn request() -> StorageQuery {
    StorageQuery {
        spec: QuerySpec::equal("amount", json!(17)),
        descriptor: Descriptor {
            kind: "items".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "amount".into(),
                shape: Shape::U64,
            }],
        },
        semantics: QUERY_SEMANTICS_VERSION,
        selection: SelectionMode::UniformReadAndFields,
    }
}
fn bounds() -> QueryBounds {
    QueryBounds {
        max_rows: 1000,
        max_bytes: 1_000_000,
    }
}
fn fixture() -> Connection {
    scalar_fixture(Shape::U64, (0u64..128).map(|n| Some(json!(n))).collect())
}
fn scalar_fixture(shape: Shape, values: Vec<Option<Value>>) -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE resources(kind TEXT,id TEXT,revision INTEGER,data TEXT,PRIMARY KEY(kind,id)); CREATE TABLE schemas(kind TEXT PRIMARY KEY,data TEXT);").unwrap();
    super::catalog::initialize(&c).unwrap();
    let mut descriptor = request().descriptor;
    descriptor.fields[0].shape = shape.clone();
    c.execute(
        "INSERT INTO schemas VALUES (?,?)",
        params!["items", serde_json::to_string(&descriptor).unwrap()],
    )
    .unwrap();
    c.execute("UPDATE query_profile SET store='test-store'", [])
        .unwrap();
    let mut bytes = 0usize;
    let total = values.len();
    for (n, value) in values.into_iter().enumerate() {
        let object = value
            .as_ref()
            .map_or_else(|| json!({}), |v| json!({"amount":v}));
        let row = Row {
            key: Key {
                kind: "items".into(),
                id: format!("{n:03}"),
            },
            revision: 1,
            value: Some(object),
            protected: ProtectedMetadata::default(),
        };
        let text = serde_json::to_string(&row).unwrap();
        bytes += text.len();
        c.execute(
            "INSERT INTO resources VALUES (?,?,?,?)",
            params![row.key.kind, row.key.id, 1, text],
        )
        .unwrap();
        c.execute(
            "INSERT INTO query_keys VALUES (?,?,?,?)",
            params![
                row.key.kind,
                "amount",
                row.key.id,
                encode(&shape, value.as_ref()).unwrap()
            ],
        )
        .unwrap();
    }
    c.execute(
        "INSERT INTO query_kinds VALUES ('items',?,?,?,?,?)",
        params![
            total as i64,
            bytes as i64,
            bytes as i64,
            total as i64,
            1u64.to_be_bytes().as_slice()
        ],
    )
    .unwrap();
    c
}
#[test]
fn real_index_returns_complete_candidates_and_exact_binding() {
    let c = fixture();
    let q = request();
    let QueryRead::NativeCandidates {
        rows,
        admission,
        binding,
    } = query_read(&c, &q, bounds()).unwrap()
    else {
        panic!("expected actual indexed native execution")
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.id, "017");
    assert_eq!(admission.rows, 128);
    assert_eq!(binding.request, q);
    assert_eq!(binding.snapshot.store, "test-store");
}
#[test]
fn opaque_and_no_predicate_reads_use_reference_and_limits_admit_whole_kind() {
    let c = fixture();
    let mut q = request();
    q.selection = SelectionMode::ReferenceOnly;
    assert!(
        matches!(query_read(&c,&q,bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==128)
    );
    q = request();
    q.spec = QuerySpec::all();
    assert!(
        matches!(query_read(&c,&q,bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==128)
    );
    q = request();
    q.spec = q.spec.limit(1);
    assert_eq!(
        query_read(
            &c,
            &q,
            QueryBounds {
                max_rows: 10,
                ..bounds()
            }
        ),
        Err(Error::TooLarge)
    );
    c.execute("UPDATE query_kinds SET row_bytes=1000001", [])
        .unwrap();
    assert_eq!(query_read(&c, &q, bounds()), Err(Error::TooLarge));
}
#[test]
fn planner_failure_falls_back_but_selected_execution_failure_propagates() {
    let c = fixture();
    let q = request();
    c.execute("DROP INDEX query_keys_value", []).unwrap();
    assert!(
        matches!(query_read(&c,&q,bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==128)
    );
    c.execute(
        "CREATE INDEX query_keys_value ON query_keys(kind,field,encoded,id)",
        [],
    )
    .unwrap();
    c.execute("UPDATE resources SET data='broken' WHERE id='017'", [])
        .unwrap();
    assert_eq!(query_read(&c, &q, bounds()), Err(Error::Storage));
}
#[test]
fn ranges_leave_other_predicates_residual_and_descriptor_must_match_catalog() {
    let c = fixture();
    let mut q = request();
    q.spec = QuerySpec::all()
        .compare("amount", CompareOp::Ge, json!(120))
        .compare("amount", CompareOp::Lt, json!(123));
    let QueryRead::NativeCandidates { rows, .. } = query_read(&c, &q, bounds()).unwrap() else {
        panic!("range should use index")
    };
    assert_eq!(
        rows.len(),
        8,
        "one leading predicate yields complete superset"
    );
    q.descriptor.version = 2;
    assert_eq!(query_read(&c, &q, bounds()), Err(Error::Storage));
}

fn native_values(c: &Connection, query: &StorageQuery) -> Vec<Option<Value>> {
    let QueryRead::NativeCandidates { rows, .. } = query_read(c, query, bounds()).unwrap() else {
        panic!("expected native path for {:?}", query.spec);
    };
    rows.into_iter()
        .map(|row| row.value.unwrap().get("amount").cloned())
        .collect()
}

#[test]
fn native_absence_null_and_ranges_keep_distinct_semantics() {
    let shape = Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::U64))));
    let values = [
        None,
        Some(Value::Null),
        Some(json!(0)),
        Some(json!(u64::MAX)),
    ];
    let c = scalar_fixture(
        shape.clone(),
        values.iter().cloned().cycle().take(128).collect(),
    );
    let mut q = request();
    q.descriptor.fields[0].shape = shape;
    q.spec = QuerySpec::absent("amount");
    assert_eq!(native_values(&c, &q), vec![None; 32]);
    q.spec = QuerySpec::equal("amount", Value::Null);
    assert_eq!(native_values(&c, &q), vec![Some(Value::Null); 32]);
    q.spec = QuerySpec::all().compare("amount", CompareOp::Eq, Value::Null);
    assert_eq!(native_values(&c, &q), vec![Some(Value::Null); 32]);
    q.spec = QuerySpec::all().compare("amount", CompareOp::Lt, json!(u64::MAX));
    assert_eq!(
        native_values(&c, &q),
        vec![Some(json!(0)); 32],
        "range excludes missing and null"
    );
    q.spec = QuerySpec::all().compare("amount", CompareOp::Ge, json!(u64::MAX));
    assert_eq!(native_values(&c, &q), vec![Some(json!(u64::MAX)); 32]);
    q.spec = QuerySpec::all();
    q.spec.comparisons.push(Comparison {
        field: "amount".into(),
        op: CompareOp::Ne,
        value: Value::Null,
        absent: true,
    });
    let candidates = native_values(&c, &q);
    assert_eq!(candidates.len(), 96);
    assert!(candidates.iter().all(Option::is_some));
    // Ordinary inequality remains reference; a missing value does not mean null.
    q.spec = QuerySpec::all().compare("amount", CompareOp::Ne, Value::Null);
    assert!(
        matches!(query_read(&c,&q,bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==128)
    );
}

#[test]
fn native_unsigned_float_and_unicode_boundaries_are_exact() {
    let cases = [
        (
            Shape::U64,
            vec![json!(0), json!(i64::MAX as u64 + 1), json!(u64::MAX)],
            json!(i64::MAX as u64 + 1),
        ),
        (
            Shape::I64,
            vec![json!(i64::MIN), json!(-1), json!(i64::MAX)],
            json!(-1),
        ),
        (
            Shape::String,
            vec![json!("a"), json!("a\0"), json!("雪")],
            json!("a\0"),
        ),
        (
            Shape::F64,
            vec![json!(-1.0), json!(0.0), json!(1.0)],
            json!(0),
        ),
    ];
    for (shape, values, pivot) in cases {
        let c = scalar_fixture(
            shape.clone(),
            values.iter().cloned().map(Some).cycle().take(129).collect(),
        );
        let mut q = request();
        q.descriptor.fields[0].shape = shape;
        for op in [
            CompareOp::Eq,
            CompareOp::Lt,
            CompareOp::Le,
            CompareOp::Gt,
            CompareOp::Ge,
        ] {
            q.spec = QuerySpec::all().compare("amount", op, pivot.clone());
            let actual = native_values(&c, &q);
            let allowed = match op {
                CompareOp::Eq => &values[1..2],
                CompareOp::Lt => &values[..1],
                CompareOp::Le => &values[..2],
                CompareOp::Gt => &values[2..],
                CompareOp::Ge => &values[1..],
                _ => unreachable!(),
            };
            assert_eq!(actual.len(), 43 * allowed.len());
            assert!(actual.iter().all(|v| allowed.contains(v.as_ref().unwrap())));
        }
    }
}

#[test]
fn metadata_corruption_is_not_an_optional_planner_failure() {
    for sql in [
        "UPDATE query_kinds SET row_count=-1",
        "UPDATE query_kinds SET live_count=129",
        "UPDATE query_kinds SET generation=x'01'",
        "UPDATE query_kinds SET row_bytes=1",
        "UPDATE query_profile SET encoding_version=2",
        "UPDATE query_profile SET store=''",
        "DELETE FROM query_kinds",
    ] {
        let c = fixture();
        c.execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        c.execute(sql, []).unwrap();
        assert_eq!(
            query_read(&c, &request(), bounds()),
            Err(Error::Storage),
            "{sql}"
        );
    }
}
