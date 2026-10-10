//! Differential storage-boundary tests against the frozen whole-ledger model.
use super::accounting::{LedgerBytes, entry_for_root, entry_for_work};
use super::incremental::*;
use super::*;
use std::cell::RefCell;

struct Reader {
    ledger: WorkLedger,
    coherence: ReadFence,
    epochs: RetryEpochs,
    reads: RefCell<Vec<String>>,
    candidates: RefCell<Vec<String>>,
    cached_header: WorkHeader,
    cached_roots: BTreeMap<String, RootAccount>,
}
impl Reader {
    fn empty() -> Self {
        Self {
            ledger: WorkLedger::default(),
            coherence: ReadFence::new(),
            epochs: RetryEpochs::default(),
            reads: RefCell::new(vec![]),
            candidates: RefCell::new(vec![]),
            cached_header: WorkHeader {
                limits: None,
                retry_epochs: RetryEpochs::default(),
                accounting: LedgerBytes::empty(None).unwrap(),
            },
            cached_roots: BTreeMap::new(),
        }
    }
    // Fixture import only: build summaries once, outside measured transitions.
    fn refresh(&mut self) {
        let mut accounting = LedgerBytes::empty(self.ledger.limits.as_ref()).unwrap();
        let mut roots = BTreeMap::<String, RootAccount>::new();
        for (id, record) in &self.ledger.work {
            accounting
                .replace_work(None, Some(entry_for_work(id, record).unwrap()))
                .unwrap();
            let root = roots
                .entry(record.pending.cause.root.clone())
                .or_insert(RootAccount {
                    used: 0,
                    epoch: record.pending.cause.retry_epoch,
                    members: 0,
                    incomplete: 0,
                });
            root.used += record.attempts;
            root.members += 1;
            root.incomplete += usize::from(!super::retention::completed(record));
        }
        for (id, used) in &self.ledger.roots {
            accounting
                .replace_root(None, Some(entry_for_root(id, *used).unwrap()))
                .unwrap();
        }
        self.cached_header = WorkHeader {
            limits: self.ledger.limits.clone(),
            retry_epochs: self.epochs,
            accounting,
        };
        self.cached_roots = roots;
        self.coherence = ReadFence::new();
    }
    fn publish(&mut self, delta: &WorkDelta) {
        assert!(self.coherence.same_context(delta.coherence()));
        assert_eq!(&self.header().unwrap(), delta.before_header());
        for (id, before) in delta.record_preconditions() {
            assert_eq!(self.ledger.work.get(id), before);
        }
        for (id, before) in delta.root_preconditions() {
            assert_eq!(self.cached_roots.get(id), before);
        }
        self.ledger.limits = delta.header().limits.clone();
        for (id, _, after) in delta.records() {
            self.ledger.work.insert(id.to_owned(), after.clone());
        }
        for (id, _, after) in delta.roots() {
            self.ledger.roots.insert(id.to_owned(), after.used);
            self.cached_roots.insert(id.to_owned(), *after);
        }
        self.cached_header = delta.header().clone();
        self.coherence = ReadFence::new();
    }
}
impl WorkRead for Reader {
    fn coherence(&self) -> ReadFence {
        self.coherence.clone()
    }
    fn header(&self) -> Result<WorkHeader> {
        Ok(self.cached_header.clone())
    }
    fn record(&self, id: &str) -> Result<Option<WorkRecord>> {
        self.reads.borrow_mut().push(id.into());
        Ok(self.ledger.work.get(id).cloned())
    }
    fn root(&self, id: &str) -> Result<Option<RootAccount>> {
        Ok(self.cached_roots.get(id).copied())
    }
    fn next_candidate(&self, now: u64, after_id: Option<&str>) -> Result<Option<WorkRecord>> {
        let record = self
            .ledger
            .work
            .iter()
            .filter(|(id, _)| after_id.is_none_or(|after| id.as_str() > after))
            .find(|(_, r)| match r.state {
                WorkState::Pending => {
                    r.due <= now
                        || now.saturating_sub(r.pending.cause.started_at)
                            >= self.ledger.limits.as_ref().unwrap().max_age_seconds
                }
                WorkState::Leased { until, .. } => until <= now,
                _ => false,
            })
            .map(|(_, r)| r.clone());
        if let Some(record) = &record {
            self.candidates.borrow_mut().push(record.pending.id.clone());
        }
        Ok(record)
    }
}
fn pending(id: &str) -> PendingWork {
    let invocation = Invocation {
        retry_epoch: 0,
        kind: "test".into(),
        id: "target".into(),
        expected: Some(1),
        idempotency: id.into(),
        operation: Operation::Action {
            name: "touch".into(),
            input: json!({}),
        },
    };
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
        not_before: None,
        delivery_profile: DeliveryProfile::AtLeastOnce,
        service_key: "service".into(),
        payload: WorkPayload::Action(serde_json::to_value(invocation).unwrap()),
    }
}
#[test]
fn incremental_enqueue_matches_frozen_storage_boundary_and_duplicate_identity() {
    let mut reader = Reader::empty();
    let mut old = reference::WorkLedger::default();
    for work in [vec![pending("a"), pending("b")], vec![pending("a")]] {
        let delta = prepare_enqueue(&reader, &ReactionLimits::default(), work.clone()).unwrap();
        if !old.records().is_empty() {
            assert_eq!(delta.records().count(), 0);
            assert_eq!(delta.roots().count(), 0);
        }
        old.enqueue(&ReactionLimits::default(), work).unwrap();
        old.validate_retry_epochs(reader.epochs).unwrap();
        reader.publish(&delta);
        assert_eq!(
            serde_json::to_value(&reader.ledger).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        assert_eq!(
            reader.header().unwrap().accounting,
            delta.header().accounting
        );
    }
    let mut changed = pending("a");
    changed.version += 1;
    assert!(matches!(
        prepare_enqueue(&reader, &ReactionLimits::default(), vec![changed]),
        Err(Error::IdentityMismatch)
    ));
    let delta = prepare_update(&Reader::empty(), WorkUpdate::Claim { now: 10 }).unwrap();
    assert_eq!(delta.result(), &WorkResult::Idle);
    assert_eq!(delta.into_result(), WorkResult::Idle);
}

fn step(
    reader: &mut Reader,
    old: &mut reference::WorkLedger,
    update: WorkUpdate,
) -> Result<WorkResult> {
    let before = serde_json::to_value(&reader.ledger).unwrap();
    let mut candidate = old.clone();
    let expected = candidate.apply(update.clone()).and_then(|result| {
        candidate.validate_retry_epochs(reader.epochs)?;
        Ok(result)
    });
    let actual = prepare_update(reader, update);
    match actual {
        Ok(delta) => {
            assert_eq!(expected.as_ref(), Ok(delta.result()));
            reader.publish(&delta);
            *old = candidate;
            assert_eq!(
                reader.header().unwrap().accounting,
                delta.header().accounting
            );
            assert_eq!(
                serde_json::to_value(&reader.ledger).unwrap(),
                serde_json::to_value(&old).unwrap()
            );
            Ok(delta.into_result())
        }
        Err(error) => {
            assert_eq!(Err(error.clone()), expected);
            assert_eq!(serde_json::to_value(&reader.ledger).unwrap(), before);
            Err(error)
        }
    }
}
fn seeded(work: Vec<PendingWork>, limits: ReactionLimits) -> (Reader, reference::WorkLedger) {
    let mut reader = Reader::empty();
    let delta = prepare_enqueue(&reader, &limits, work.clone()).unwrap();
    reader.publish(&delta);
    let mut old = reference::WorkLedger::default();
    old.enqueue(&limits, work).unwrap();
    old.validate_retry_epochs(reader.epochs).unwrap();
    (reader, old)
}
fn claim(reader: &mut Reader, old: &mut reference::WorkLedger, now: u64) -> WorkClaim {
    let WorkResult::Claimed(claim) = step(reader, old, WorkUpdate::Claim { now }).unwrap() else {
        panic!("missing claim")
    };
    *claim
}
fn source() -> Row {
    Row {
        key: Key {
            kind: "test".into(),
            id: "1".into(),
        },
        revision: 1,
        value: Some(json!({})),
        protected: ProtectedMetadata::default(),
    }
}
#[test]
fn all_five_updates_preserve_composed_revision_generation_and_root_summaries() {
    let (mut reader, mut old) = seeded(vec![pending("a")], ReactionLimits::default());
    let first = claim(&mut reader, &mut old, 10);
    step(
        &mut reader,
        &mut old,
        WorkUpdate::Finish {
            claim: first.key(),
            now: 11,
            outcome: WorkOutcome::Retry,
        },
    )
    .unwrap();
    assert_eq!(
        step(&mut reader, &mut old, WorkUpdate::Claim { now: 11 }).unwrap(),
        WorkResult::Idle
    );
    let second = claim(&mut reader, &mut old, 12);
    step(
        &mut reader,
        &mut old,
        WorkUpdate::Finish {
            claim: second.key(),
            now: 13,
            outcome: WorkOutcome::Done,
        },
    )
    .unwrap();
    assert_eq!(
        reader.root("root").unwrap().unwrap(),
        RootAccount {
            used: 2,
            epoch: 0,
            members: 1,
            incomplete: 0
        }
    );

    let mut work = pending("source");
    work.payload = WorkPayload::Source(source());
    let (mut reader, mut old) = seeded(vec![work], ReactionLimits::default());
    let parent = claim(&mut reader, &mut old, 10);
    step(
        &mut reader,
        &mut old,
        WorkUpdate::Materialize {
            claim: parent.key(),
            now: 11,
            children: vec![pending("child"), pending("child")],
        },
    )
    .unwrap();
    assert_eq!(reader.ledger.work["source"].revision, 2);
    assert_eq!(reader.ledger.work["child"].revision, 0);
    assert_eq!(reader.root("root").unwrap().unwrap().members, 2);
    let child = claim(&mut reader, &mut old, 11);
    step(
        &mut reader,
        &mut old,
        WorkUpdate::Finish {
            claim: child.key(),
            now: 12,
            outcome: WorkOutcome::Stop(StopReason::Denied),
        },
    )
    .unwrap();

    for profile in [
        DeliveryProfile::AtLeastOnce,
        DeliveryProfile::ProviderDeduplicated,
        DeliveryProfile::ReconcileBeforeRetry,
    ] {
        for outcome in [
            DeliveryOutcome::Accepted,
            DeliveryOutcome::Retryable,
            DeliveryOutcome::Permanent,
            DeliveryOutcome::Unknown,
            DeliveryOutcome::TimedOut,
            DeliveryOutcome::Panicked,
        ] {
            let mut work = pending("notification");
            work.payload = WorkPayload::Notification {
                source: source(),
                payload: json!({}),
            };
            work.delivery_profile = profile.clone();
            let (mut reader, mut old) = seeded(vec![work], ReactionLimits::default());
            let first = claim(&mut reader, &mut old, 10);
            step(
                &mut reader,
                &mut old,
                WorkUpdate::DeliveryStarted {
                    claim: first.key(),
                    now: 11,
                },
            )
            .unwrap();
            let repeated = prepare_update(
                &reader,
                WorkUpdate::DeliveryStarted {
                    claim: first.key(),
                    now: 11,
                },
            )
            .unwrap();
            assert_eq!(repeated.result(), &WorkResult::Changed);
            assert_eq!(repeated.records().count(), 0);
            assert_eq!(repeated.roots().count(), 0);
            step(
                &mut reader,
                &mut old,
                WorkUpdate::DeliveryStarted {
                    claim: first.key(),
                    now: 11,
                },
            )
            .unwrap();
            assert_eq!(reader.ledger.work["notification"].revision, 2);
            step(
                &mut reader,
                &mut old,
                WorkUpdate::DeliveryFinished {
                    claim: first.key(),
                    now: 12,
                    outcome,
                },
            )
            .unwrap();
            assert_eq!(reader.ledger.work["notification"].revision, 3);
        }
    }
}
#[test]
fn id_ordered_prefix_and_late_overflow_are_atomic() {
    let limits = ReactionLimits {
        max_age_seconds: 20,
        ..ReactionLimits::default()
    };
    let mut delayed = pending("a");
    delayed.not_before = Some(100);
    let mut unknown = pending("b");
    unknown.payload = WorkPayload::Notification {
        source: source(),
        payload: json!({}),
    };
    unknown.delivery_profile = DeliveryProfile::ReconcileBeforeRetry;
    let (mut reader, _) = seeded(vec![delayed, unknown, pending("c")], limits);
    {
        let ledger = &mut reader.ledger;
        ledger.work.get_mut("b").unwrap().state = WorkState::Leased {
            until: 30,
            generation: 1,
            resolution_only: None,
        };
        ledger.work.get_mut("b").unwrap().generation = 1;
        ledger.work.get_mut("b").unwrap().delivery = Some(DeliveryOutcome::Unknown);
    }
    reader.refresh();
    let mut old = serde_json::from_value(serde_json::to_value(&reader.ledger).unwrap()).unwrap();
    let claim = claim(&mut reader, &mut old, 30);
    assert_eq!(claim.work.pending.id, "c");
    assert!(claim.resolution_only);
    assert_eq!(
        reader.ledger.work["a"].state,
        WorkState::Stopped(StopReason::Age)
    );
    assert_eq!(
        reader.ledger.work["b"].state,
        WorkState::AwaitingReconciliation
    );

    let (mut reader, _) = seeded(vec![pending("a"), pending("b")], ReactionLimits::default());
    reader.ledger.work.get_mut("a").unwrap().due = 10000;
    reader.ledger.work.get_mut("b").unwrap().revision = u64::MAX;
    reader.refresh();
    let mut old = serde_json::from_value(serde_json::to_value(&reader.ledger).unwrap()).unwrap();
    assert_eq!(
        step(&mut reader, &mut old, WorkUpdate::Claim { now: 4000 }),
        Err(Error::TooLarge)
    );
    assert_eq!(reader.ledger.work["a"].state, WorkState::Pending);
}

fn enqueue_step(
    reader: &mut Reader,
    old: &mut reference::WorkLedger,
    limits: &ReactionLimits,
    work: Vec<PendingWork>,
) -> Result<()> {
    let mut candidate = old.clone();
    let expected = candidate
        .enqueue(limits, work.clone())
        .and_then(|()| candidate.validate_retry_epochs(reader.epochs));
    match prepare_enqueue(reader, limits, work) {
        Ok(delta) => {
            assert_eq!(expected, Ok(()));
            reader.publish(&delta);
            *old = candidate;
            assert_eq!(
                serde_json::to_value(&reader.ledger).unwrap(),
                serde_json::to_value(old).unwrap()
            );
            Ok(())
        }
        Err(error) => {
            assert_eq!(expected, Err(error.clone()));
            Err(error)
        }
    }
}
#[test]
fn epoch_policy_capacity_and_materialization_errors_preserve_original_precedence() {
    let (mut reader, mut old) = seeded(vec![pending("a")], ReactionLimits::default());
    let before = serde_json::to_value(&reader.ledger).unwrap();
    let changed = ReactionLimits {
        lease_seconds: 31,
        ..ReactionLimits::default()
    };
    assert!(matches!(
        enqueue_step(&mut reader, &mut old, &changed, vec![]),
        Err(Error::Unsupported(_))
    ));
    let mut mismatched = pending("b");
    mismatched.cause.retry_epoch = 1;
    assert_eq!(
        enqueue_step(
            &mut reader,
            &mut old,
            &ReactionLimits::default(),
            vec![mismatched]
        ),
        Err(Error::Storage)
    );
    let mut malformed = pending("b");
    malformed.payload = WorkPayload::Action(json!({}));
    assert_eq!(
        enqueue_step(
            &mut reader,
            &mut old,
            &ReactionLimits::default(),
            vec![malformed]
        ),
        Err(Error::Storage)
    );
    assert_eq!(serde_json::to_value(&reader.ledger).unwrap(), before);
    let mut mismatch = pending("b");
    mismatch.cause.retry_epoch = 1;
    let mut changed = pending("a");
    changed.version = 99;
    assert_eq!(
        enqueue_step(
            &mut reader,
            &mut old,
            &ReactionLimits::default(),
            vec![mismatch, changed]
        ),
        Err(Error::IdentityMismatch)
    );
    let limits = ReactionLimits {
        max_records: 1,
        ..ReactionLimits::default()
    };
    let mut parent = pending("parent");
    parent.payload = WorkPayload::Source(source());
    let (mut reader, mut old) = seeded(vec![parent], limits);
    let claim = claim(&mut reader, &mut old, 10);
    assert_eq!(
        step(
            &mut reader,
            &mut old,
            WorkUpdate::Materialize {
                claim: claim.key(),
                now: 11,
                children: vec![pending("child")]
            }
        ),
        Err(Error::Overloaded)
    );
    let mut bad = pending("child");
    bad.definition = "other".into();
    assert!(matches!(
        step(
            &mut reader,
            &mut old,
            WorkUpdate::Materialize {
                claim: claim.key(),
                now: 11,
                children: vec![bad]
            }
        ),
        Err(Error::Invalid { .. })
    ));
    assert_eq!(
        step(
            &mut reader,
            &mut old,
            WorkUpdate::Finish {
                claim: ClaimKey {
                    id: claim.work.pending.id.clone(),
                    generation: claim.key().generation + 1
                },
                now: 11,
                outcome: WorkOutcome::Done
            }
        ),
        Err(Error::Conflict)
    );
    assert_eq!(
        step(
            &mut reader,
            &mut old,
            WorkUpdate::DeliveryStarted {
                claim: claim.key(),
                now: 11
            }
        ),
        Err(Error::Conflict)
    );
}
#[test]
fn local_transition_does_not_read_or_emit_unrelated_terminal_work() {
    let limits = ReactionLimits {
        max_records: 10000,
        max_bytes: 16 * 1024 * 1024,
        ..ReactionLimits::default()
    };
    let (mut reader, _) = seeded(vec![pending("active")], limits);
    for index in 0..2000 {
        let mut record = reader.ledger.work["active"].clone();
        record.pending.id = format!("terminal-{index:04}");
        record.state = WorkState::Done;
        reader.ledger.work.insert(record.pending.id.clone(), record);
    }
    reader.refresh();
    reader.reads.borrow_mut().clear();
    reader.candidates.borrow_mut().clear();
    let delta = prepare_update(&reader, WorkUpdate::Claim { now: 10 }).unwrap();
    assert_eq!(&*reader.candidates.borrow(), &["active"]);
    assert!(reader.reads.borrow().iter().all(|id| id == "active"));
    assert_eq!(
        delta.records().map(|(id, _, _)| id).collect::<Vec<_>>(),
        vec!["active"]
    );
    reader.publish(&delta);
    let WorkResult::Claimed(claim) = delta.into_result() else {
        panic!("missing claim")
    };
    reader.reads.borrow_mut().clear();
    reader.candidates.borrow_mut().clear();
    let delta = prepare_update(
        &reader,
        WorkUpdate::Finish {
            claim: claim.key(),
            now: 11,
            outcome: WorkOutcome::Done,
        },
    )
    .unwrap();
    assert_eq!(&*reader.reads.borrow(), &["active"]);
    assert!(reader.candidates.borrow().is_empty());
    assert_eq!(delta.records().count(), 1);
    assert_eq!(delta.roots().count(), 1);
}
#[test]
fn empty_policy_and_invalid_epoch_headers_fail_without_work_reads() {
    let mut reader = Reader::empty();
    let idle = prepare_update(
        &reader,
        WorkUpdate::Finish {
            claim: ClaimKey {
                id: "missing".into(),
                generation: u64::MAX,
            },
            now: 0,
            outcome: WorkOutcome::Done,
        },
    )
    .unwrap();
    assert_eq!(idle.into_result(), WorkResult::Idle);
    assert!(reader.reads.borrow().is_empty());
    reader.cached_header.retry_epochs = RetryEpochs {
        current: 0,
        admission_floor: 1,
        replay_floor: 0,
    };
    assert!(matches!(
        prepare_update(&reader, WorkUpdate::Claim { now: 0 }),
        Err(Error::Invalid { .. })
    ));
    assert!(reader.reads.borrow().is_empty());
}

#[test]
fn bundle_editor_checks_claim_and_completion_revision_before_enqueue() {
    let (mut reader, mut old) = seeded(vec![pending("a")], ReactionLimits::default());
    let claim = claim(&mut reader, &mut old, 10);
    let mut edit = WorkEdit::new(&reader).unwrap();
    assert_eq!(edit.claim_retry_epoch(&claim.key(), 11).unwrap(), 0);
    edit.complete(claim.key(), 11).unwrap();
    edit.enqueue(&ReactionLimits::default(), vec![pending("b")])
        .unwrap();
    let delta = edit.finish().unwrap();
    old.apply(WorkUpdate::Finish {
        claim: claim.key(),
        now: 11,
        outcome: WorkOutcome::Done,
    })
    .unwrap();
    old.enqueue(&ReactionLimits::default(), vec![pending("b")])
        .unwrap();
    old.validate_retry_epochs(reader.epochs).unwrap();
    reader.publish(&delta);
    assert_eq!(
        serde_json::to_value(&reader.ledger).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    assert_eq!(reader.ledger.work["a"].revision, 2);
    let mut work = reader.ledger.work["b"].clone();
    work.state = WorkState::Leased {
        until: 30,
        generation: 1,
        resolution_only: Some(StopReason::Age),
    };
    work.generation = 1;
    reader.ledger.work.insert("b".into(), work);
    reader.refresh();
    let mut edit = WorkEdit::new(&reader).unwrap();
    assert_eq!(
        edit.claim_retry_epoch(
            &ClaimKey {
                id: "b".into(),
                generation: 1
            },
            11
        ),
        Err(Error::Conflict)
    );
    reader.ledger.work.get_mut("b").unwrap().revision = u64::MAX;
    reader.refresh();
    let mut edit = WorkEdit::new(&reader).unwrap();
    assert_eq!(
        edit.complete(
            ClaimKey {
                id: "b".into(),
                generation: 1
            },
            11
        ),
        Err(Error::TooLarge)
    );
}

#[test]
fn failed_public_editor_cannot_publish_partial_enqueue_after_ignored_error() {
    let (reader, _) = seeded(vec![pending("a")], ReactionLimits::default());
    let mut edit = WorkEdit::new(&reader).unwrap();
    let mut changed = pending("a");
    changed.version = 99;
    assert_eq!(
        edit.enqueue(&ReactionLimits::default(), vec![pending("new"), changed]),
        Err(Error::IdentityMismatch)
    );
    assert!(matches!(edit.finish(), Err(Error::IdentityMismatch)));
    assert!(!reader.ledger.work.contains_key("new"));
}
