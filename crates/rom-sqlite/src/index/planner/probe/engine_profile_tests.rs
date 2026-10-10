use super::{recognized, recognized_for_engine};
use crate::index::planner::predicate::Predicate;
use crate::index::{catalog, encode};
use rom::{
    CompareOp, Descriptor, FieldDescriptor, QuerySpec, SelectionMode, Shape, StorageQuery, json,
};
use rusqlite::{Connection, params, params_from_iter, types::Value};

fn fixture() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE resources(kind TEXT,id TEXT,revision INTEGER,data TEXT,PRIMARY KEY(kind,id)); CREATE TABLE schemas(kind TEXT PRIMARY KEY,data TEXT);").unwrap();
    catalog::initialize(&c).unwrap();
    let shape = Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::U64))));
    for n in 0u64..128 {
        let id = format!("{n:03}");
        c.execute("INSERT INTO resources VALUES ('items',?,1,'{}')", [&id])
            .unwrap();
        c.execute(
            "INSERT INTO query_keys VALUES ('items','amount',?,?)",
            params![id, encode(&shape, Some(&json!(n))).unwrap()],
        )
        .unwrap();
    }
    c
}

fn request(spec: QuerySpec) -> StorageQuery {
    StorageQuery {
        descriptor: Descriptor {
            kind: "items".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "amount".into(),
                shape: Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::U64)))),
            }],
        },
        spec,
        semantics: rom::QUERY_SEMANTICS_VERSION,
        selection: SelectionMode::UniformReadAndFields,
    }
}

fn details(c: &Connection, sql: &str, parameters: &[Value]) -> Vec<String> {
    let mut statement = c.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
    let rows = statement
        .query_map(params_from_iter(parameters), |r| r.get(3))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();
    assert!(rows.len() <= 16 && rows.iter().all(|s| s.len() <= 512));
    rows
}

#[test]
fn known_engine_profiles_keep_exact_covering_and_lookup_shapes() {
    let c = fixture();
    let predicate =
        Predicate::for_request(&request(QuerySpec::equal("amount", json!(17)))).remove(0);
    let sql = predicate.probe_sql();
    let mut parameters = predicate.parameters.clone();
    parameters.push(Value::Integer(17));
    for version in ["3.53.2", "3.53.4"] {
        assert_eq!(
            recognized_for_engine(&c, &sql, &parameters, false, version),
            Some(true)
        );
        assert_eq!(
            recognized_for_engine(&c, &sql, &parameters, true, version),
            Some(false),
            "covering count alone is not materialization"
        );
    }
    let plan = predicate.materialization();
    for version in ["3.53.2", "3.53.4"] {
        assert_eq!(
            recognized_for_engine(&c, &plan.sql, &plan.parameters, true, version),
            Some(true)
        );
    }
    assert_eq!(
        recognized(&c, &plan.sql, &plan.parameters, true),
        Some(true)
    );
}

#[test]
fn unreviewed_engine_profiles_keep_reference_fallback() {
    let c = fixture();
    let predicate =
        Predicate::for_request(&request(QuerySpec::equal("amount", json!(17)))).remove(0);
    let plan = predicate.materialization();
    for version in [
        "",
        "3.53.1",
        "3.53.3",
        "3.53.5",
        "3.54.0",
        "3.53.4\n",
        "3.53.4-custom",
    ] {
        assert_eq!(
            recognized_for_engine(&c, &plan.sql, &plan.parameters, true, version),
            None
        );
    }
}

#[test]
fn unexpected_physical_plans_remain_unrecognized() {
    let c = fixture();
    let predicate =
        Predicate::for_request(&request(QuerySpec::equal("amount", json!(17)))).remove(0);
    let plan = predicate.materialization();
    assert_ne!(
        recognized_for_engine(
            &c,
            &(plan.sql.clone() + " ORDER BY r.data"),
            &plan.parameters,
            true,
            rusqlite::version()
        ),
        Some(true)
    );
    c.execute("DROP INDEX query_keys_value", []).unwrap();
    assert_ne!(
        recognized_for_engine(&c, &plan.sql, &plan.parameters, true, rusqlite::version()),
        Some(true)
    );
}

#[test]
fn canonical_query_profile_eqp_witness() {
    let c = fixture();
    let source_id: String = c
        .query_row("SELECT sqlite_source_id()", [], |r| r.get(0))
        .unwrap();
    let mut specs = vec![
        ("equal", QuerySpec::equal("amount", json!(17))),
        ("absent", QuerySpec::absent("amount")),
        ("null", QuerySpec::equal("amount", rom::Value::Null)),
    ];
    specs.extend(
        [
            (CompareOp::Lt, "lt"),
            (CompareOp::Le, "le"),
            (CompareOp::Gt, "gt"),
            (CompareOp::Ge, "ge"),
        ]
        .into_iter()
        .map(|(op, name)| (name, QuerySpec::all().compare("amount", op, json!(17)))),
    );
    for (case, spec) in specs {
        let predicate = Predicate::for_request(&request(spec)).remove(0);
        let probe_sql = predicate.probe_sql();
        let mut parameters = predicate.parameters.clone();
        parameters.push(Value::Integer(17));
        let plan = predicate.materialization();
        for (materialization, sql, parameters) in [
            (false, probe_sql, parameters),
            (true, plan.sql, plan.parameters),
        ] {
            let rows = details(&c, &sql, &parameters);
            println!(
                "ROM_SQLITE_EQP_WITNESS {}",
                serde_json::json!({"engine": rusqlite::version(), "source_id": source_id, "case": case, "materialization": materialization, "sql": sql, "details": rows})
            );
            assert_eq!(
                recognized(&c, &sql, &parameters, materialization),
                Some(true)
            );
        }
    }
}
