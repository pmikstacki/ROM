use super::accounting::*;
use super::*;

#[test]
fn empty_envelope_matches_canonical_serialization_with_and_without_limits() {
    for limits in [None, Some(ReactionLimits::default())] {
        let ledger = WorkLedger {
            limits: limits.clone(),
            ..WorkLedger::default()
        };
        let bytes = serde_json::to_vec(&ledger).unwrap().len();
        assert_eq!(
            LedgerBytes::empty(limits.as_ref())
                .unwrap()
                .totals()
                .unwrap(),
            EntryBytes {
                current: bytes,
                reserved: bytes,
            }
        );
    }
}

fn record(id: &str) -> WorkRecord {
    WorkRecord {
        pending: PendingWork {
            id: id.into(),
            cause: Cause {
                retry_epoch: 0,
                root: "root\"\n雪".into(),
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
            payload: WorkPayload::Action(json!({"value":null,"escaped":"\\\"雪"})),
        },
        state: WorkState::Pending,
        attempts: 0,
        generation: 0,
        revision: 0,
        due: 10,
        delivery: None,
    }
}
fn canonical_totals(ledger: &WorkLedger) -> EntryBytes {
    // Independent original whole-ledger reservation oracle, not the helper.
    let mut reserved = ledger.clone();
    for used in reserved.roots.values_mut() {
        *used = u32::MAX;
    }
    for work in reserved.work.values_mut() {
        work.attempts = u32::MAX;
        work.generation = u64::MAX;
        work.revision = u64::MAX;
        work.due = u64::MAX;
        work.state = WorkState::Leased {
            until: u64::MAX,
            generation: u64::MAX,
            resolution_only: Some(StopReason::DefinitionChanged),
        };
        work.delivery = Some(DeliveryOutcome::Retryable);
    }
    EntryBytes {
        current: serde_json::to_vec(ledger).unwrap().len(),
        reserved: serde_json::to_vec(&reserved).unwrap().len(),
    }
}
#[test]
fn affected_entries_match_whole_serialization_through_insert_replace_remove() {
    let mut ledger = WorkLedger {
        limits: Some(ReactionLimits::default()),
        ..WorkLedger::default()
    };
    let mut bytes = LedgerBytes::empty(ledger.limits.as_ref()).unwrap();
    for id in ["one\"\n雪", "two", "three\\"] {
        let work = record(id);
        bytes
            .replace_work(None, Some(entry_for_work(id, &work).unwrap()))
            .unwrap();
        ledger.work.insert(id.into(), work);
        assert_eq!(bytes.totals().unwrap(), canonical_totals(&ledger));
    }
    for (id, used) in [("root\"\n雪", 0), ("other", u32::MAX)] {
        bytes
            .replace_root(None, Some(entry_for_root(id, used).unwrap()))
            .unwrap();
        ledger.roots.insert(id.into(), used);
        assert_eq!(bytes.totals().unwrap(), canonical_totals(&ledger));
    }
    let id = "two";
    let before = entry_for_work(id, &ledger.work[id]).unwrap();
    ledger.work.get_mut(id).unwrap().state = WorkState::AwaitingReconciliation;
    bytes
        .replace_work(
            Some(before),
            Some(entry_for_work(id, &ledger.work[id]).unwrap()),
        )
        .unwrap();
    assert_eq!(bytes.totals().unwrap(), canonical_totals(&ledger));
    let old = entry_for_root("other", u32::MAX).unwrap();
    ledger.roots.insert("other".into(), 12);
    bytes
        .replace_root(Some(old), Some(entry_for_root("other", 12).unwrap()))
        .unwrap();
    assert_eq!(bytes.totals().unwrap(), canonical_totals(&ledger));
    while let Some((id, work)) = ledger.work.pop_first() {
        bytes
            .replace_work(Some(entry_for_work(&id, &work).unwrap()), None)
            .unwrap();
        assert_eq!(bytes.totals().unwrap(), canonical_totals(&ledger));
    }
    while let Some((id, used)) = ledger.roots.pop_first() {
        bytes
            .replace_root(Some(entry_for_root(&id, used).unwrap()), None)
            .unwrap();
        assert_eq!(bytes.totals().unwrap(), canonical_totals(&ledger));
    }
}

fn assembled(ledger: &WorkLedger) -> LedgerBytes {
    let mut bytes = LedgerBytes::empty(ledger.limits.as_ref()).unwrap();
    for (id, work) in &ledger.work {
        bytes
            .replace_work(None, Some(entry_for_work(id, work).unwrap()))
            .unwrap();
    }
    for (id, used) in &ledger.roots {
        bytes
            .replace_root(None, Some(entry_for_root(id, *used).unwrap()))
            .unwrap();
    }
    bytes
}
#[test]
fn every_state_delivery_and_optional_floor_retains_exact_reservation() {
    let stops = [
        StopReason::Depth,
        StopReason::WorkBudget,
        StopReason::Attempts,
        StopReason::Age,
        StopReason::Fanout,
        StopReason::Denied,
        StopReason::Conflict,
        StopReason::Invalid,
        StopReason::Missing,
        StopReason::DefinitionChanged,
        StopReason::CallbackPanicked,
        StopReason::Unavailable,
        StopReason::DeliveryPermanent,
    ];
    let mut states = vec![
        WorkState::Pending,
        WorkState::AwaitingReconciliation,
        WorkState::Done,
        WorkState::Leased {
            until: 50,
            generation: 2,
            resolution_only: None,
        },
        WorkState::Leased {
            until: u64::MAX,
            generation: u64::MAX,
            resolution_only: Some(StopReason::DefinitionChanged),
        },
    ];
    states.extend(stops.into_iter().map(WorkState::Stopped));
    for state in states {
        for delivery in [
            None,
            Some(DeliveryOutcome::Accepted),
            Some(DeliveryOutcome::Retryable),
            Some(DeliveryOutcome::Permanent),
            Some(DeliveryOutcome::Unknown),
            Some(DeliveryOutcome::TimedOut),
            Some(DeliveryOutcome::Panicked),
        ] {
            for floor in [None, Some(0), Some(u64::MAX)] {
                let mut work = record("雪\"\\\n");
                work.state = state.clone();
                work.delivery = delivery.clone();
                work.pending.not_before = floor;
                work.attempts = u32::MAX;
                work.generation = u64::MAX;
                work.revision = u64::MAX;
                work.due = u64::MAX;
                let mut ledger = WorkLedger::default();
                ledger
                    .roots
                    .insert(work.pending.cause.root.clone(), u32::MAX);
                ledger.work.insert(work.pending.id.clone(), work);
                assert_eq!(
                    assembled(&ledger).totals().unwrap(),
                    canonical_totals(&ledger)
                );
            }
        }
    }
    let mut wire = serde_json::to_value(record("null-floor")).unwrap();
    wire["pending"]["not_before"] = Value::Null;
    let work: WorkRecord = serde_json::from_value(wire).unwrap();
    assert_eq!(work.pending.not_before, None);
    let mut ledger = WorkLedger::default();
    ledger.work.insert(work.pending.id.clone(), work);
    assert_eq!(
        assembled(&ledger).totals().unwrap(),
        canonical_totals(&ledger)
    );
}
#[test]
fn root_count_does_not_consume_record_capacity_and_byte_bounds_match_ledger() {
    for max_records in [1, 2, usize::MAX] {
        for max_bytes in [1, 600, 900, 4096, usize::MAX] {
            let limits = ReactionLimits {
                max_records,
                max_bytes,
                ..ReactionLimits::default()
            };
            let mut ledger = WorkLedger {
                limits: Some(limits.clone()),
                ..WorkLedger::default()
            };
            for id in ["a", "b"] {
                ledger.work.insert(id.into(), record(id));
            }
            for id in ["r1", "r2", "r3"] {
                ledger.roots.insert(id.into(), 0);
            }
            assert_eq!(
                assembled(&ledger).check_bounds(&limits),
                ledger.check_bounds()
            );
        }
    }
    let mut ledger = WorkLedger {
        limits: Some(ReactionLimits {
            max_records: 1,
            ..ReactionLimits::default()
        }),
        work: [("a".into(), record("a"))].into(),
        roots: [("r1".into(), 0), ("r2".into(), 0)].into(),
    };
    for _ in 0..8 {
        let total = assembled(&ledger).totals().unwrap();
        let cap = total.current.max(total.reserved);
        if ledger.limits.as_ref().unwrap().max_bytes == cap {
            break;
        }
        ledger.limits.as_mut().unwrap().max_bytes = cap;
    }
    let bytes = assembled(&ledger);
    let limits = ledger.limits.as_ref().unwrap();
    assert_eq!(bytes.totals().unwrap().reserved, limits.max_bytes);
    bytes.check_bounds(limits).unwrap();
    let mut too_small = ledger.clone();
    too_small.limits.as_mut().unwrap().max_bytes -= 1;
    assert_eq!(
        assembled(&too_small).check_bounds(too_small.limits.as_ref().unwrap()),
        Err(Error::Overloaded)
    );
    assert!(matches!(
        bytes.check_bounds(too_small.limits.as_ref().unwrap()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn rejected_contribution_changes_are_atomic() {
    let mut bytes = LedgerBytes::empty(None).unwrap();
    let before = bytes.clone();
    assert_eq!(
        bytes.replace_work(
            Some(EntryBytes {
                current: 8,
                reserved: 8
            }),
            None
        ),
        Err(Error::Storage)
    );
    assert_eq!(bytes, before);
    assert_eq!(
        bytes.replace_root(
            None,
            Some(EntryBytes {
                current: 0,
                reserved: 8
            })
        ),
        Err(Error::Storage)
    );
    assert_eq!(bytes, before);
    assert_eq!(
        bytes.replace_work(
            None,
            Some(EntryBytes {
                current: usize::MAX,
                reserved: usize::MAX
            })
        ),
        Err(Error::TooLarge)
    );
    assert_eq!(bytes, before);
    let entry = entry_for_work("a", &record("a")).unwrap();
    bytes.replace_work(None, Some(entry)).unwrap();
    let populated = bytes.clone();
    let wrong = EntryBytes {
        current: entry.current - 1,
        reserved: entry.reserved,
    };
    assert_eq!(bytes.replace_work(Some(wrong), None), Err(Error::Storage));
    assert_eq!(bytes, populated);
    let underflow = EntryBytes {
        current: entry.current,
        reserved: entry.reserved + 1,
    };
    assert_eq!(
        bytes.replace_work(Some(underflow), Some(entry)),
        Err(Error::Storage)
    );
    assert_eq!(bytes, populated);
    assert_eq!(
        bytes.replace_root(
            None,
            Some(EntryBytes {
                current: 8,
                reserved: usize::MAX
            })
        ),
        Err(Error::TooLarge)
    );
    assert_eq!(bytes, populated);
    bytes.replace_work(Some(entry), Some(entry)).unwrap();
    assert_eq!(bytes, populated);
    bytes.replace_work(None, None).unwrap();
    assert_eq!(bytes, populated);
}

#[test]
fn source_and_notification_payloads_keep_frozen_json_contributions() {
    let row = Row {
        key: Key {
            kind: "kind\"雪".into(),
            id: "id\\\n".into(),
        },
        revision: u64::MAX,
        value: Some(json!({"nested": [null, false, 0, "", "雪\\\"\n"]})),
        protected: ProtectedMetadata::default(),
    };
    for payload in [
        WorkPayload::Source(row.clone()),
        WorkPayload::Notification {
            source: row,
            payload: json!({"null":null,"empty":{}}),
        },
    ] {
        let mut work = record("payload");
        work.pending.payload = payload;
        work.pending.not_before = Some(0);
        let mut ledger = WorkLedger {
            limits: Some(ReactionLimits::default()),
            ..WorkLedger::default()
        };
        ledger.roots.insert(work.pending.cause.root.clone(), 0);
        ledger.work.insert(work.pending.id.clone(), work);
        assert_eq!(
            assembled(&ledger).totals().unwrap(),
            canonical_totals(&ledger)
        );
    }
}

#[test]
fn persisted_accounting_parts_reconstruct_checked_canonical_totals() {
    let mut ledger = WorkLedger {
        limits: Some(ReactionLimits::default()),
        ..WorkLedger::default()
    };
    ledger.work.insert("work".into(), record("work"));
    ledger.roots.insert("root".into(), 0);
    let bytes = assembled(&ledger);
    let wire = serde_json::to_vec(&bytes.parts()).unwrap();
    let parts: WorkAccounting = serde_json::from_slice(&wire).unwrap();
    assert_eq!(
        LedgerBytes::from_parts(ledger.limits.as_ref(), parts).unwrap(),
        bytes
    );
    let bad = WorkAccounting {
        work: MapAccounting {
            entries: 0,
            current_entries_bytes: 1,
            reserved_entries_bytes: 1,
        },
        roots: MapAccounting::default(),
    };
    assert!(matches!(
        LedgerBytes::from_parts(None, bad),
        Err(Error::Storage)
    ));
    let overflow = WorkAccounting {
        work: MapAccounting {
            entries: 1,
            current_entries_bytes: usize::MAX,
            reserved_entries_bytes: usize::MAX,
        },
        roots: MapAccounting::default(),
    };
    assert!(matches!(
        LedgerBytes::from_parts(Some(&ReactionLimits::default()), overflow),
        Err(Error::TooLarge)
    ));
}
