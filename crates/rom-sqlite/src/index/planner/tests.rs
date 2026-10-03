use crate::index::{catalog, encode, query::query_read};
use rom::*;
use rusqlite::{Connection, params};

fn descriptor() -> Descriptor {
    Descriptor {
        kind: "probe-items".into(),
        version: 1,
        fields: vec![
            FieldDescriptor {
                name: "amount".into(),
                shape: Shape::U64,
            },
            FieldDescriptor {
                name: "category".into(),
                shape: Shape::String,
            },
            FieldDescriptor {
                name: "group".into(),
                shape: Shape::U64,
            },
        ],
    }
}

fn request(spec: QuerySpec) -> StorageQuery {
    StorageQuery {
        descriptor: descriptor(),
        spec,
        semantics: QUERY_SEMANTICS_VERSION,
        selection: SelectionMode::UniformReadAndFields,
    }
}

fn bounds() -> QueryBounds {
    QueryBounds {
        max_rows: 10_000,
        max_bytes: 16_000_000,
    }
}

fn fixture(count: usize) -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE resources(kind TEXT,id TEXT,revision INTEGER,data TEXT,PRIMARY KEY(kind,id)); CREATE TABLE schemas(kind TEXT PRIMARY KEY,data TEXT);").unwrap();
    catalog::initialize(&c).unwrap();
    let descriptor = descriptor();
    c.execute(
        "INSERT INTO schemas VALUES (?,?)",
        params![descriptor.kind, serde_json::to_string(&descriptor).unwrap()],
    )
    .unwrap();
    let mut bytes = 0usize;
    for n in 0..count {
        let value =
            json!({"amount":n,"category":if n % 10 == 0 {"cold"} else {"hot"},"group":n % 16});
        let row = Row {
            key: Key {
                kind: descriptor.kind.clone(),
                id: format!("{n:05}"),
            },
            revision: 1,
            value: Some(value.clone()),
            protected: ProtectedMetadata::default(),
        };
        let text = serde_json::to_string(&row).unwrap();
        bytes += text.len();
        c.execute(
            "INSERT INTO resources VALUES (?,?,1,?)",
            params![row.key.kind, row.key.id, text],
        )
        .unwrap();
        for field in &descriptor.fields {
            c.execute(
                "INSERT INTO query_keys VALUES (?,?,?,?)",
                params![
                    row.key.kind,
                    field.name,
                    row.key.id,
                    encode(&field.shape, value.get(&field.name)).unwrap()
                ],
            )
            .unwrap();
        }
    }
    c.execute(
        "INSERT INTO query_kinds VALUES (?,?,?,?,?,?)",
        params![
            descriptor.kind,
            count as i64,
            bytes as i64,
            bytes as i64,
            count as i64,
            1u64.to_be_bytes().as_slice()
        ],
    )
    .unwrap();
    c
}

#[test]
fn broad_skew_and_ranges_choose_reference_after_whole_kind_admission() {
    let c = fixture(1024);
    for spec in [
        QuerySpec::equal("category", json!("hot")),
        QuerySpec::all().compare("amount", CompareOp::Ge, json!(0)),
    ] {
        assert!(
            matches!(query_read(&c,&request(spec),bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==1024)
        );
    }
}

#[test]
fn conjunction_order_does_not_hide_the_eight_row_candidate() {
    let c = fixture(1024);
    for spec in [
        QuerySpec::all()
            .compare("category", CompareOp::Eq, json!("hot"))
            .compare("amount", CompareOp::Ge, json!(1016)),
        QuerySpec::all()
            .compare("amount", CompareOp::Ge, json!(1016))
            .compare("category", CompareOp::Eq, json!("hot")),
    ] {
        let QueryRead::NativeCandidates { rows, .. } =
            query_read(&c, &request(spec), bounds()).unwrap()
        else {
            panic!("selective conjunct should use native candidates")
        };
        assert_eq!(rows.len(), 8);
        assert!(
            rows.iter()
                .all(|row| row.value.as_ref().unwrap()["amount"].as_u64().unwrap() >= 1016)
        );
    }
}

#[test]
fn medium_selectivity_above_128_matches_can_still_choose_native() {
    let c = fixture(8192);
    let QueryRead::NativeCandidates { rows, .. } =
        query_read(&c, &request(QuerySpec::equal("group", json!(0))), bounds()).unwrap()
    else {
        panic!("512 of 8192 matches should remain eligible")
    };
    assert_eq!(rows.len(), 512);
}

#[test]
fn duplicate_predicates_do_not_consume_the_bounded_selection_slots() {
    let c = fixture(1024);
    let mut spec = QuerySpec::all();
    for _ in 0..20 {
        spec = spec.compare("category", CompareOp::Eq, json!("hot"));
    }
    spec = spec.compare("amount", CompareOp::Ge, json!(1016));
    let QueryRead::NativeCandidates { rows, .. } =
        query_read(&c, &request(spec), bounds()).unwrap()
    else {
        panic!("duplicate broad predicates cannot hide the later selective predicate")
    };
    assert_eq!(rows.len(), 8);
}

#[test]
fn more_than_four_distinct_predicates_use_the_documented_bounded_prefix() {
    let c = fixture(1024);
    let mut spec = QuerySpec::all();
    for lower in 0..4 {
        spec = spec.compare("amount", CompareOp::Ge, json!(lower));
    }
    spec = spec.compare("amount", CompareOp::Ge, json!(1016));
    assert!(
        matches!(query_read(&c,&request(spec),bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==1024)
    );
}

#[test]
fn unknown_physical_index_layout_disables_optional_probes() {
    let c = fixture(1024);
    c.execute("DROP INDEX query_keys_value", []).unwrap();
    assert!(
        matches!(query_read(&c,&request(QuerySpec::equal("amount",json!(17))),bounds()).unwrap(),QueryRead::Reference{rows} if rows.len()==1024)
    );
}

#[cfg(feature = "test-support")]
#[test]
fn saturated_probe_does_not_truncate_forced_native_materialization() {
    let c = fixture(1024);
    let q = request(
        QuerySpec::all()
            .compare("amount", CompareOp::Ge, json!(0))
            .limit(1),
    );
    let observation =
        crate::index::query::query_read_observed(&c, &q, bounds(), crate::QueryExecution::Native)
            .unwrap();
    let QueryRead::NativeCandidates { rows, .. } = observation.read else {
        panic!("saturation changes estimated cost, not native capability")
    };
    assert_eq!(rows.len(), 1024);
}

#[cfg(feature = "test-support")]
#[test]
fn probes_distinguish_exact_counts_saturation_and_reference_control() {
    let c = fixture(1024);
    for (lower, statements, visited) in [(1008, 1, 16), (1007, 2, 34)] {
        let q = request(QuerySpec::all().compare("amount", CompareOp::Ge, json!(lower)));
        let observed = crate::index::query::query_read_observed(
            &c,
            &q,
            bounds(),
            crate::QueryExecution::Automatic,
        )
        .unwrap();
        assert_eq!(observed.metrics.probe_statements, statements);
        assert_eq!(observed.metrics.probe_rows, visited);
        assert!(observed.metrics.probe_vm_steps > 0);
        assert_eq!(observed.metrics.strategy, QueryStrategy::NativeCandidates);
        let reference = crate::index::query::query_read_observed(
            &c,
            &q,
            bounds(),
            crate::QueryExecution::Reference,
        )
        .unwrap();
        assert_eq!(reference.metrics.probe_statements, 0);
        assert_eq!(reference.metrics.probe_rows, 0);
        assert_eq!(reference.metrics.probe_vm_steps, 0);
    }
}

#[cfg(feature = "test-support")]
#[test]
fn first_pass_selectivity_avoids_expanding_a_common_earlier_predicate() {
    let c = fixture(1024);
    let q = request(
        QuerySpec::all()
            .compare("category", CompareOp::Eq, json!("hot"))
            .compare("amount", CompareOp::Ge, json!(1016)),
    );
    let observed = crate::index::query::query_read_observed(
        &c,
        &q,
        bounds(),
        crate::QueryExecution::Automatic,
    )
    .unwrap();
    assert_eq!(observed.metrics.probe_statements, 2);
    assert_eq!(observed.metrics.probe_rows, 25);
    assert_eq!(observed.metrics.decoded_rows, 8);
}

#[cfg(feature = "test-support")]
#[test]
fn empty_candidate_stops_probing_and_opaque_reads_do_not_probe() {
    let c = fixture(1024);
    let mut q = request(
        QuerySpec::all()
            .compare("amount", CompareOp::Eq, json!(9999))
            .compare("category", CompareOp::Eq, json!("hot")),
    );
    let observed = crate::index::query::query_read_observed(
        &c,
        &q,
        bounds(),
        crate::QueryExecution::Automatic,
    )
    .unwrap();
    assert_eq!(observed.metrics.probe_statements, 1);
    assert_eq!(observed.metrics.probe_rows, 0);
    assert_eq!(observed.metrics.decoded_rows, 0);
    q.selection = SelectionMode::ReferenceOnly;
    let opaque =
        crate::index::query::query_read_observed(&c, &q, bounds(), crate::QueryExecution::Native)
            .unwrap();
    assert_eq!(opaque.metrics.strategy, QueryStrategy::Reference);
    assert_eq!(opaque.metrics.probe_statements, 0);
    assert_eq!(opaque.metrics.probe_rows, 0);
}

#[cfg(feature = "test-support")]
#[test]
fn expanded_probe_shrinks_against_an_exact_candidate_and_keeps_total_work_bounded() {
    let c = fixture(8192);
    let q = request(
        QuerySpec::all()
            .compare("group", CompareOp::Eq, json!(0))
            .compare("amount", CompareOp::Ge, json!(0)),
    );
    let observed = crate::index::query::query_read_observed(
        &c,
        &q,
        bounds(),
        crate::QueryExecution::Automatic,
    )
    .unwrap();
    assert_eq!(observed.metrics.probe_statements, 4);
    assert_eq!(observed.metrics.probe_rows, 1058);
    assert_eq!(observed.metrics.decoded_rows, 512);
    let mut spec = QuerySpec::all();
    for lower in 0..4 {
        spec = spec.compare("amount", CompareOp::Ge, json!(lower));
    }
    let broad = crate::index::query::query_read_observed(
        &c,
        &request(spec),
        bounds(),
        crate::QueryExecution::Automatic,
    )
    .unwrap();
    assert_eq!(broad.metrics.strategy, QueryStrategy::Reference);
    assert_eq!(broad.metrics.probe_statements, 8);
    assert_eq!(broad.metrics.probe_rows, 16456);
}
