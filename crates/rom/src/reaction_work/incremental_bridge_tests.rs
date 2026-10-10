//! Canonical import, coherent delta application and active-ID selection.
use super::incremental::*;
use super::incremental_bridge::WorkImage;
use super::*;

fn work(id: &str, tag: &str) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            retry_epoch: 0,
            root: "root".into(),
            parent: None,
            depth: 0,
            started_at: 10,
            path: vec![],
        },
        definition: "test".into(),
        version: 1,
        service_key: "service".into(),
        delivery_profile: DeliveryProfile::AtLeastOnce,
        not_before: None,
        payload: WorkPayload::Source(Row {
            key: Key {
                kind: "test".into(),
                id: "target".into(),
            },
            revision: 1,
            value: Some(json!({"tag":tag})),
            protected: ProtectedMetadata::default(),
        }),
    }
}
fn ledger(tag: &str) -> WorkLedger {
    let mut ledger = WorkLedger::default();
    ledger
        .enqueue(&ReactionLimits::default(), vec![work("work", tag)])
        .unwrap();
    ledger
}
#[test]
fn canonical_import_roundtrips_and_rejects_inconsistent_roots_and_epochs() {
    let canonical = ledger("A");
    let image = WorkImage::from_ledger(canonical.clone(), RetryEpochs::default()).unwrap();
    assert_eq!(
        serde_json::to_value(image.canonical()).unwrap(),
        serde_json::to_value(&canonical).unwrap()
    );
    let mut wire = serde_json::to_value(&canonical).unwrap();
    wire["roots"]["root"] = json!(1);
    assert!(matches!(
        WorkImage::from_ledger(
            serde_json::from_value(wire).unwrap(),
            RetryEpochs::default()
        ),
        Err(Error::Storage)
    ));
    let mut wire = serde_json::to_value(&canonical).unwrap();
    wire["work"]["work"]["pending"]["cause"]["retry_epoch"] = json!(1);
    assert!(matches!(
        WorkImage::from_ledger(
            serde_json::from_value(wire).unwrap(),
            RetryEpochs::default()
        ),
        Err(Error::Storage)
    ));
}
#[test]
fn delta_applies_once_and_retains_canonical_done_history() {
    let mut image = WorkImage::from_ledger(ledger("A"), RetryEpochs::default()).unwrap();
    let first = prepare_update(&image, WorkUpdate::Claim { now: 10 }).unwrap();
    let stale = prepare_update(&image, WorkUpdate::Claim { now: 10 }).unwrap();
    let claim = match image.apply(first).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("claim: {other:?}"),
    };
    let before = serde_json::to_value(image.canonical()).unwrap();
    assert_eq!(image.apply(stale), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(image.canonical()).unwrap(), before);
    let delta = prepare_update(
        &image,
        WorkUpdate::Materialize {
            claim: claim.key(),
            now: 11,
            children: vec![],
        },
    )
    .unwrap();
    assert_eq!(image.apply(delta), Ok(WorkResult::Changed));
    let canonical = image.canonical();
    assert_eq!(canonical.records()[0].state, WorkState::Done);
    assert_eq!(canonical.records()[0].revision, 2);
    assert!(image.next_candidate(10000, None).unwrap().is_none());
    canonical.validate_archive().unwrap();
}
#[test]
fn equal_accounting_does_not_authorize_foreign_prior_records() {
    let first = WorkImage::from_ledger(ledger("A"), RetryEpochs::default()).unwrap();
    let mut other = WorkImage::from_ledger(ledger("B"), RetryEpochs::default()).unwrap();
    assert_eq!(first.header().unwrap(), other.header().unwrap());
    let delta = prepare_update(&first, WorkUpdate::Claim { now: 10 }).unwrap();
    let before = serde_json::to_value(other.canonical()).unwrap();
    assert_eq!(other.apply(delta), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(other.canonical()).unwrap(), before);
}
#[test]
fn terminal_history_is_absent_from_active_id_selection() {
    let mut canonical = WorkLedger::default();
    canonical
        .enqueue(
            &ReactionLimits::default(),
            (0..500).map(|i| work(&format!("{i:04}"), "A")).collect(),
        )
        .unwrap();
    let mut wire = serde_json::to_value(canonical).unwrap();
    for i in 0..499 {
        wire["work"][format!("{i:04}")]["state"] = json!("Done");
    }
    let canonical = serde_json::from_value(wire).unwrap();
    let mut image = WorkImage::from_ledger(canonical, RetryEpochs::default()).unwrap();
    assert_eq!(image.active_count(), 1);
    assert_eq!(
        image.next_candidate(10, None).unwrap().unwrap().pending.id,
        "0499"
    );
    let delta = prepare_update(&image, WorkUpdate::Claim { now: 10 }).unwrap();
    let claim = match image.apply(delta).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("claim: {other:?}"),
    };
    let delta = prepare_update(
        &image,
        WorkUpdate::Materialize {
            claim: claim.key(),
            now: 11,
            children: vec![],
        },
    )
    .unwrap();
    image.apply(delta).unwrap();
    assert_eq!(image.active_count(), 0);
    assert_eq!(image.canonical().records().len(), 500);
}

#[test]
fn unchanged_write_set_still_checks_exact_record_read_preconditions() {
    let first = WorkImage::from_ledger(ledger("A"), RetryEpochs::default()).unwrap();
    let mut other = WorkImage::from_ledger(ledger("B"), RetryEpochs::default()).unwrap();
    assert_eq!(first.header().unwrap(), other.header().unwrap());
    let delta =
        prepare_enqueue(&first, &ReactionLimits::default(), vec![work("work", "A")]).unwrap();
    assert_eq!(delta.records().count(), 0);
    assert_eq!(delta.roots().count(), 0);
    let before = serde_json::to_value(other.canonical()).unwrap();
    assert_eq!(other.apply(delta), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(other.canonical()).unwrap(), before);
}

#[test]
fn idle_delta_cannot_cross_equal_accounting_candidate_contexts() {
    let mut deferred = serde_json::to_value(ledger("A")).unwrap();
    deferred["work"]["work"]["due"] = json!(12);
    let mut eligible = deferred.clone();
    eligible["work"]["work"]["due"] = json!(11);
    let first = WorkImage::from_ledger(
        serde_json::from_value(deferred).unwrap(),
        RetryEpochs::default(),
    )
    .unwrap();
    let mut other = WorkImage::from_ledger(
        serde_json::from_value(eligible).unwrap(),
        RetryEpochs::default(),
    )
    .unwrap();
    assert_eq!(first.header().unwrap(), other.header().unwrap());
    let delta = prepare_update(&first, WorkUpdate::Claim { now: 11 }).unwrap();
    assert_eq!(delta.result(), &WorkResult::Idle);
    assert_eq!(delta.records().count(), 0);
    let before = serde_json::to_value(other.canonical()).unwrap();
    assert_eq!(other.apply(delta), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(other.canonical()).unwrap(), before);
}

#[test]
fn successful_idle_application_invalidates_earlier_context() {
    let mut wire = serde_json::to_value(ledger("A")).unwrap();
    wire["work"]["work"]["due"] = json!(12);
    let mut image = WorkImage::from_ledger(
        serde_json::from_value(wire).unwrap(),
        RetryEpochs::default(),
    )
    .unwrap();
    let first = prepare_update(&image, WorkUpdate::Claim { now: 11 }).unwrap();
    let stale = prepare_update(&image, WorkUpdate::Claim { now: 11 }).unwrap();
    let before = serde_json::to_value(image.canonical()).unwrap();
    assert_eq!(image.apply(first), Ok(WorkResult::Idle));
    assert_eq!(serde_json::to_value(image.canonical()).unwrap(), before);
    assert_eq!(image.apply(stale), Err(Error::Conflict));
}

struct AlteredRead<'a> {
    image: &'a WorkImage,
    record: Option<WorkRecord>,
    root: Option<RootAccount>,
}
impl WorkRead for AlteredRead<'_> {
    fn coherence(&self) -> ReadFence {
        self.image.coherence()
    }
    fn header(&self) -> Result<WorkHeader> {
        self.image.header()
    }
    fn record(&self, id: &str) -> Result<Option<WorkRecord>> {
        if id == "work" && self.record.is_some() {
            Ok(self.record.clone())
        } else {
            self.image.record(id)
        }
    }
    fn root(&self, id: &str) -> Result<Option<RootAccount>> {
        if id == "root" && self.root.is_some() {
            Ok(self.root)
        } else {
            self.image.root(id)
        }
    }
    fn next_candidate(&self, now: u64, after: Option<&str>) -> Result<Option<WorkRecord>> {
        self.image.next_candidate(now, after)
    }
}

#[test]
fn same_context_noop_checks_readonly_record_fact() {
    let mut image = WorkImage::from_ledger(ledger("A"), RetryEpochs::default()).unwrap();
    let record = WorkImage::from_ledger(ledger("B"), RetryEpochs::default())
        .unwrap()
        .record("work")
        .unwrap()
        .unwrap();
    let read = AlteredRead {
        image: &image,
        record: Some(record),
        root: None,
    };
    let delta =
        prepare_enqueue(&read, &ReactionLimits::default(), vec![work("work", "B")]).unwrap();
    assert_eq!(delta.records().count(), 0);
    assert_eq!(delta.record_preconditions().count(), 1);
    assert!(image.coherence().same_context(delta.coherence()));
    let before = serde_json::to_value(image.canonical()).unwrap();
    assert_eq!(image.apply(delta), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(image.canonical()).unwrap(), before);
}

#[test]
fn same_context_checks_exact_root_fact_before_publication() {
    let mut image = WorkImage::from_ledger(ledger("A"), RetryEpochs::default()).unwrap();
    let root = RootAccount {
        members: 2,
        ..image.root("root").unwrap().unwrap()
    };
    let read = AlteredRead {
        image: &image,
        record: None,
        root: Some(root),
    };
    let delta =
        prepare_enqueue(&read, &ReactionLimits::default(), vec![work("second", "A")]).unwrap();
    assert_eq!(delta.root_preconditions().count(), 1);
    assert!(image.coherence().same_context(delta.coherence()));
    let before = serde_json::to_value(image.canonical()).unwrap();
    assert_eq!(image.apply(delta), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(image.canonical()).unwrap(), before);
}

#[test]
fn native_import_checks_all_stored_projections_without_repair() {
    let image = WorkImage::from_ledger(ledger("A"), RetryEpochs::default()).unwrap();
    let header = image.header().unwrap();
    let records: BTreeMap<_, _> = image
        .records()
        .map(|(id, record)| (id.to_owned(), record.clone()))
        .collect();
    let roots: BTreeMap<_, _> = image
        .roots()
        .map(|(id, root)| (id.to_owned(), *root))
        .collect();
    let active: BTreeSet<_> = image.active_ids().map(str::to_owned).collect();
    let imported = WorkImage::from_native(
        header.clone(),
        records.clone(),
        roots.clone(),
        active.clone(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(imported.canonical()).unwrap(),
        serde_json::to_value(image.canonical()).unwrap()
    );
    let mut wrong_header = header.clone();
    wrong_header.accounting =
        super::accounting::LedgerBytes::empty(header.limits.as_ref()).unwrap();
    assert!(matches!(
        WorkImage::from_native(wrong_header, records.clone(), roots.clone(), active.clone()),
        Err(Error::Storage)
    ));
    let mut wrong_roots = roots.clone();
    wrong_roots.get_mut("root").unwrap().members += 1;
    assert!(matches!(
        WorkImage::from_native(header.clone(), records.clone(), wrong_roots, active.clone()),
        Err(Error::Storage)
    ));
    assert!(matches!(
        WorkImage::from_native(
            header.clone(),
            records.clone(),
            roots.clone(),
            BTreeSet::new()
        ),
        Err(Error::Storage)
    ));
    assert!(matches!(
        WorkImage::from_native(header, BTreeMap::new(), roots, active),
        Err(Error::Storage)
    ));
}
