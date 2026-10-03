#![cfg(feature = "test-support")]
use rom::*;
use rom_sqlite::{QueryExecution, Sqlite};

fn request() -> StorageQuery {
    StorageQuery {
        spec: QuerySpec::equal("amount", json!(0)),
        descriptor: Descriptor {
            kind: "observations".into(),
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
fn fixture(count: u64) -> Sqlite {
    let db = Sqlite::open(":memory:").unwrap();
    db.register(&[request().descriptor]).unwrap();
    for n in 0..count {
        let identity = format!("seed-{n}");
        db.commit(&Bundle {
            expected: None,
            receipt: Receipt {
                identity: identity.clone(),
                fingerprint: identity,
                retry_epoch: 0,
                replay_version: None,
                row: Row {
                    key: Key {
                        kind: "observations".into(),
                        id: format!("{n:03}"),
                    },
                    revision: 1,
                    value: Some(json!({"amount":n})),
                    protected: ProtectedMetadata::default(),
                },
            },
            changed: true,
            effects: vec![],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
        })
        .unwrap();
    }
    db
}
fn rows(read: &QueryRead) -> &[Row] {
    match read {
        QueryRead::Reference { rows } | QueryRead::NativeCandidates { rows, .. } => rows,
    }
}
#[test]
fn observed_modes_share_real_reads_and_count_materialization() {
    let db = fixture(128);
    let request = request();
    let automatic = db
        .query_read_observed(&request, bounds(), QueryExecution::Automatic)
        .unwrap();
    assert_eq!(automatic.read, db.query_read(&request, bounds()).unwrap());
    let native = db
        .query_read_observed(&request, bounds(), QueryExecution::Native)
        .unwrap();
    let reference = db
        .query_read_observed(&request, bounds(), QueryExecution::Reference)
        .unwrap();
    assert_eq!(automatic.read, native.read);
    assert_eq!(native.metrics.strategy, QueryStrategy::NativeCandidates);
    assert_eq!(reference.metrics.strategy, QueryStrategy::Reference);
    assert_eq!(native.metrics.decoded_rows, 1);
    assert_eq!(reference.metrics.decoded_rows, 128);
    for observation in [&native, &reference] {
        assert_eq!(
            observation.metrics.decoded_bytes,
            rows(&observation.read)
                .iter()
                .map(|r| serde_json::to_vec(r).unwrap().len())
                .sum::<usize>()
        );
        assert!(observation.metrics.vm_steps > 0);
    }
    assert!(native.metrics.vm_steps < reference.metrics.vm_steps);
    let matching: Vec<_> = rows(&reference.read)
        .iter()
        .filter(|r| r.value.as_ref().unwrap()["amount"] == json!(0))
        .cloned()
        .collect();
    assert_eq!(rows(&native.read), matching);
    let mut empty = request;
    empty.spec = QuerySpec::equal("amount", json!(999));
    let observed = db
        .query_read_observed(&empty, bounds(), QueryExecution::Native)
        .unwrap();
    assert_eq!(observed.metrics.decoded_rows, 0);
    assert_eq!(observed.metrics.decoded_bytes, 0);
    assert!(observed.metrics.vm_steps > 0);
}
#[test]
fn forcing_changes_only_cost_and_keeps_capability_and_admission() {
    let small = fixture(1);
    let mut request = request();
    assert_eq!(
        small
            .query_read_observed(&request, bounds(), QueryExecution::Automatic)
            .unwrap()
            .metrics
            .strategy,
        QueryStrategy::Reference
    );
    assert_eq!(
        small
            .query_read_observed(&request, bounds(), QueryExecution::Native)
            .unwrap()
            .metrics
            .strategy,
        QueryStrategy::NativeCandidates
    );
    request.selection = SelectionMode::ReferenceOnly;
    assert_eq!(
        small
            .query_read_observed(&request, bounds(), QueryExecution::Native)
            .unwrap()
            .metrics
            .strategy,
        QueryStrategy::Reference
    );
    request.selection = SelectionMode::UniformReadAndFields;
    request.spec = QuerySpec::all();
    assert_eq!(
        small
            .query_read_observed(&request, bounds(), QueryExecution::Native)
            .unwrap()
            .metrics
            .strategy,
        QueryStrategy::Reference
    );
    let large = fixture(128);
    request.spec = QuerySpec::equal("amount", json!(0)).limit(1);
    for mode in [
        QueryExecution::Automatic,
        QueryExecution::Reference,
        QueryExecution::Native,
    ] {
        assert_eq!(
            large
                .query_read_observed(
                    &request,
                    QueryBounds {
                        max_bytes: 1,
                        ..bounds()
                    },
                    mode
                )
                .unwrap_err(),
            Error::TooLarge
        );
        let mut unsupported = request.clone();
        unsupported.semantics = QUERY_SEMANTICS_VERSION + 1;
        let fallback = large
            .query_read_observed(&unsupported, bounds(), mode)
            .unwrap();
        assert_eq!(fallback.metrics.strategy, QueryStrategy::Reference);
        assert_eq!(fallback.metrics.probe_statements, 0);
        assert_eq!(
            large
                .query_read_observed(
                    &request,
                    QueryBounds {
                        max_rows: 1,
                        ..bounds()
                    },
                    mode
                )
                .unwrap_err(),
            Error::TooLarge
        );
        let mut invalid = request.clone();
        invalid.descriptor.version = 2;
        assert_eq!(
            large
                .query_read_observed(&invalid, bounds(), mode)
                .unwrap_err(),
            Error::Storage
        );
    }
}
