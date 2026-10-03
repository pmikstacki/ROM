use super::*;
use crate::*;
use std::sync::atomic::{AtomicUsize, Ordering};
fn request() -> StorageQuery {
    StorageQuery {
        spec: QuerySpec::equal("n", json!(1)),
        descriptor: Descriptor {
            kind: "items".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "n".into(),
                shape: Shape::U64,
            }],
        },
        semantics: 1,
        selection: SelectionMode::UniformReadAndFields,
    }
}
fn snapshot() -> QuerySnapshot {
    QuerySnapshot {
        store: "store".into(),
        generation: 1,
        profile_version: 1,
        encoding_version: 1,
    }
}
fn estimates(q: &StorageQuery) -> QueryEstimates {
    QueryEstimates {
        binding: ReadBinding {
            request: q.clone(),
            snapshot: snapshot(),
        },
        complete_candidates: true,
        reference: QueryCost {
            startup: 10,
            rows: 2,
            per_row: 2,
            bytes: 10,
            per_byte: 1,
        },
        native: QueryCost {
            startup: 1,
            rows: 1,
            per_row: 1,
            bytes: 1,
            per_byte: 1,
        },
    }
}
fn row(id: &str) -> Row {
    Row {
        key: Key {
            kind: "items".into(),
            id: id.into(),
        },
        revision: 1,
        value: Some(json!({"n":1})),
        protected: ProtectedMetadata::default(),
    }
}
fn bounds() -> QueryBounds {
    QueryBounds {
        max_rows: 10,
        max_bytes: 10000,
    }
}
fn admission(rows: &[Row]) -> KindAdmission {
    let bytes = serde_json::to_vec(&rows[0]).unwrap().len() * rows.len();
    KindAdmission {
        rows: rows.len(),
        persisted_bytes: bytes,
        canonical_bytes: bytes,
    }
}
fn native(q: &StorageQuery, rows: Vec<Row>) -> QueryRead {
    QueryRead::NativeCandidates {
        admission: admission(&rows),
        rows,
        binding: Box::new(ReadBinding {
            request: q.clone(),
            snapshot: snapshot(),
        }),
    }
}
#[test]
fn selector_requires_exact_capability_binding_and_strict_checked_saving() {
    let q = request();
    let e = estimates(&q);
    assert_eq!(
        select_query_strategy(&q, &snapshot(), Some(&e)),
        QueryStrategy::NativeCandidates
    );
    assert_eq!(
        select_query_strategy(&q, &snapshot(), None),
        QueryStrategy::Reference
    );
    let mut variants = vec![];
    let mut stale = e.clone();
    stale.binding.snapshot.generation += 1;
    variants.push(stale);
    let mut wrong = e.clone();
    wrong.binding.request.spec.limit = Some(2);
    variants.push(wrong);
    let mut incomplete = e.clone();
    incomplete.complete_candidates = false;
    variants.push(incomplete);
    let mut tie = e.clone();
    tie.native = tie.reference;
    variants.push(tie);
    let mut overflow = e.clone();
    overflow.native.rows = u64::MAX;
    overflow.native.per_row = 2;
    variants.push(overflow);
    let mut overflow = e.clone();
    overflow.reference.bytes = u64::MAX;
    overflow.reference.per_byte = 2;
    variants.push(overflow);
    for e in variants {
        assert_eq!(
            select_query_strategy(&q, &snapshot(), Some(&e)),
            QueryStrategy::Reference
        );
    }
}
#[test]
fn selector_rejects_opaque_unknown_shapes_versions_and_identity() {
    let q = request();
    let mut requests = vec![];
    let mut opaque = q.clone();
    opaque.selection = SelectionMode::ReferenceOnly;
    requests.push(opaque);
    let mut future = q.clone();
    future.semantics = 2;
    requests.push(future);
    let mut list = q.clone();
    list.descriptor.fields[0].shape = Shape::List(Box::new(Shape::U64));
    requests.push(list);
    let mut missing = q.clone();
    missing.spec = QuerySpec::equal("unknown", json!(1));
    requests.push(missing);
    for q in requests {
        assert_eq!(
            select_query_strategy(&q, &snapshot(), Some(&estimates(&q))),
            QueryStrategy::Reference
        );
    }
    for s in [
        QuerySnapshot {
            store: String::new(),
            ..snapshot()
        },
        QuerySnapshot {
            store: "x".repeat(257),
            ..snapshot()
        },
        QuerySnapshot {
            encoding_version: 2,
            ..snapshot()
        },
        QuerySnapshot {
            profile_version: 2,
            ..snapshot()
        },
    ] {
        let mut e = estimates(&q);
        e.binding.snapshot = s.clone();
        assert_eq!(
            select_query_strategy(&q, &s, Some(&e)),
            QueryStrategy::Reference
        );
    }
}
#[test]
fn scalar_wrappers_and_unreferenced_containers_are_eligible() {
    let mut q = request();
    q.descriptor.fields[0].shape = Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::F64))));
    q.descriptor.fields.push(FieldDescriptor {
        name: "other".into(),
        shape: Shape::Map(Box::new(Shape::Bool)),
    });
    assert_eq!(
        select_query_strategy(&q, &snapshot(), Some(&estimates(&q))),
        QueryStrategy::NativeCandidates
    );
}
#[test]
fn reference_preserves_validation_order_and_sorts_ids() {
    let q = request();
    let mut wrong = row("x");
    wrong.key.kind = "wrong".into();
    assert_eq!(
        validate_query_read(
            &q,
            QueryBounds {
                max_rows: 0,
                ..bounds()
            },
            QueryRead::Reference {
                rows: vec![wrong.clone()]
            }
        ),
        Err(Error::TooLarge)
    );
    assert_eq!(
        validate_query_read(
            &q,
            QueryBounds {
                max_bytes: 0,
                ..bounds()
            },
            QueryRead::Reference { rows: vec![wrong] }
        ),
        Err(Error::Storage)
    );
    assert_eq!(
        validate_query_read(
            &q,
            QueryBounds {
                max_bytes: 0,
                ..bounds()
            },
            QueryRead::Reference {
                rows: vec![row("a"), row("a")]
            }
        ),
        Err(Error::TooLarge)
    );
    assert_eq!(
        validate_query_read(
            &q,
            bounds(),
            QueryRead::Reference {
                rows: vec![row("a"), row("a")]
            }
        ),
        Err(Error::Storage)
    );
    let rows = validate_query_read(
        &q,
        bounds(),
        QueryRead::Reference {
            rows: vec![row("z"), row("a")],
        },
    )
    .unwrap();
    assert_eq!(rows[0].key.id, "a");
}
#[test]
fn native_admits_whole_kind_and_rejects_inconsistent_responses() {
    let q = request();
    let rows = vec![row("a")];
    assert_eq!(
        validate_query_read(&q, bounds(), native(&q, rows.clone())).unwrap(),
        rows
    );
    for mutate in [
        |a: &mut KindAdmission| a.rows = 11,
        |a: &mut KindAdmission| a.persisted_bytes = 10001,
        |a: &mut KindAdmission| a.canonical_bytes = 10001,
    ] {
        let mut response = native(&q, rows.clone());
        if let QueryRead::NativeCandidates { admission, .. } = &mut response {
            mutate(admission)
        };
        assert_eq!(
            validate_query_read(&q, bounds(), response),
            Err(Error::TooLarge)
        );
    }
    for mutate in [
        |a: &mut KindAdmission| a.rows = 0,
        |a: &mut KindAdmission| a.canonical_bytes = 0,
    ] {
        let mut response = native(&q, rows.clone());
        if let QueryRead::NativeCandidates { admission, .. } = &mut response {
            mutate(admission)
        };
        assert_eq!(
            validate_query_read(&q, bounds(), response),
            Err(Error::Storage)
        );
    }
    let mut opaque = q.clone();
    opaque.selection = SelectionMode::ReferenceOnly;
    assert_eq!(
        validate_query_read(&opaque, bounds(), native(&opaque, rows.clone())),
        Err(Error::Storage)
    );
    let mut wrong = native(&q, rows);
    if let QueryRead::NativeCandidates { binding, .. } = &mut wrong {
        binding.request.descriptor.version = 2;
    }
    assert_eq!(
        validate_query_read(&q, bounds(), wrong),
        Err(Error::Storage)
    );
}
#[test]
fn canonical_candidate_bytes_do_not_claim_stored_text_length() {
    let q = request();
    let mut response = native(&q, vec![row("a")]);
    if let QueryRead::NativeCandidates { admission, .. } = &mut response {
        admission.persisted_bytes = admission.canonical_bytes - 1;
    }
    assert!(validate_query_read(&q, bounds(), response).is_ok());
}
struct Legacy(AtomicUsize);
impl Storage for Legacy {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, _: &Key) -> Result<Option<Row>> {
        unreachable!()
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        assert_eq!((kind, max_rows, max_bytes), ("items", 10, 10000));
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(vec![row("a")])
    }
    fn receipt(&self, _: &str) -> Result<Option<Receipt>> {
        unreachable!()
    }
    fn commit(&self, _: &Bundle) -> Result<Receipt> {
        unreachable!()
    }
}
#[test]
fn default_read_delegates_to_one_bounded_snapshot() {
    let storage = Legacy(AtomicUsize::new(0));
    assert!(
        matches!(storage.query_read(&request(),bounds()).unwrap(),QueryRead::Reference{rows} if rows==vec![row("a")])
    );
    assert_eq!(storage.0.load(Ordering::Relaxed), 1);
}

#[test]
fn legacy_adapter_does_not_claim_runtime_ownership() {
    let storage = Legacy(AtomicUsize::new(0));
    assert!(matches!(
        storage.acquire_owner(),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn native_rejects_unknown_response_profiles_and_duplicate_candidates() {
    let q = request();
    for identity in [
        QuerySnapshot {
            store: String::new(),
            ..snapshot()
        },
        QuerySnapshot {
            store: "é".repeat(129),
            ..snapshot()
        },
        QuerySnapshot {
            profile_version: 2,
            ..snapshot()
        },
        QuerySnapshot {
            encoding_version: 2,
            ..snapshot()
        },
    ] {
        let mut response = native(&q, vec![row("a")]);
        if let QueryRead::NativeCandidates { binding, .. } = &mut response {
            binding.snapshot = identity;
        }
        assert_eq!(
            validate_query_read(&q, bounds(), response),
            Err(Error::Storage)
        );
    }
    assert_eq!(
        validate_query_read(&q, bounds(), native(&q, vec![row("a"), row("a")])),
        Err(Error::Storage)
    );
    let empty = QueryRead::NativeCandidates {
        rows: vec![],
        admission: KindAdmission {
            rows: 0,
            persisted_bytes: 0,
            canonical_bytes: 0,
        },
        binding: Box::new(ReadBinding {
            request: q.clone(),
            snapshot: snapshot(),
        }),
    };
    assert!(validate_query_read(&q, bounds(), empty).unwrap().is_empty());
}
#[test]
fn checked_score_rejects_addition_overflow_and_unrelated_snapshot_identity() {
    let q = request();
    let mut e = estimates(&q);
    e.reference.startup = u64::MAX;
    assert_eq!(
        select_query_strategy(&q, &snapshot(), Some(&e)),
        QueryStrategy::Reference
    );
    e = estimates(&q);
    e.native.startup = u64::MAX;
    assert_eq!(
        select_query_strategy(&q, &snapshot(), Some(&e)),
        QueryStrategy::Reference
    );
    e = estimates(&q);
    e.binding.snapshot.store = "other-store".into();
    assert_eq!(
        select_query_strategy(&q, &snapshot(), Some(&e)),
        QueryStrategy::Reference
    );
}

#[test]
fn public_scalar_classification_preserves_wrapped_container_boundary() {
    assert!(Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::F64)))).is_scalar());
    assert!(!Shape::Optional(Box::new(Shape::List(Box::new(Shape::U64)))).is_scalar());
    assert!(!Shape::Map(Box::new(Shape::String)).is_scalar());
}
