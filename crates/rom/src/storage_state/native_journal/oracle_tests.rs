use super::*;
use crate::storage_support::work::WorkImage;
impl StorageState {
    fn reference_journal(&mut self, b: &Bundle) -> Result<Vec<String>> {
        let mut removed = vec![];
        if b.changed {
            self.head = self.head.checked_add(1).ok_or(Error::TooLarge)?;
            let event = JournalEvent {
                position: self.head,
                identity: b.receipt.identity.clone(),
                row: b.receipt.row.clone(),
            };
            if serde_json::to_vec(&event)
                .map_err(|_| Error::Storage)?
                .len()
                > self.limits.journal_bytes
            {
                return Err(Error::TooLarge);
            }
            self.events.push(event);
            while self.events.len() > self.limits.journal_rows
                || self.events.iter().try_fold(0usize, |sum, e| {
                    sum.checked_add(serde_json::to_vec(e).map_err(|_| Error::Storage)?.len())
                        .ok_or(Error::TooLarge)
                })? > self.limits.journal_bytes
            {
                let old = self.events.remove(0);
                self.floor = old.position;
                removed.push(old.identity);
            }
        }
        Ok(removed)
    }
}

pub(super) fn bundle(id: &str, kind: &str, changed: bool) -> Bundle {
    Bundle {
        expected: None,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: id.into(),
            fingerprint: id.into(),
            row: Row {
                key: Key {
                    kind: kind.into(),
                    id: id.into(),
                },
                revision: 1,
                value: Some(json!({"id":id})),
                protected: ProtectedMetadata::default(),
            },
        },
        changed,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
struct Reader {
    image: JournalImage,
    fence: crate::storage_support::work::ReadFence,
}
impl JournalRead for Reader {
    fn coherence(&self) -> crate::storage_support::work::ReadFence {
        self.fence.clone()
    }
    fn header(&self) -> Result<MetadataHeader> {
        Ok(self.image.header().clone())
    }
    fn entry(&self, position: u64) -> Result<Option<JournalEntry>> {
        self.image
            .events()
            .iter()
            .find(|e| e.position == position)
            .map(|e| {
                Ok(JournalEntry {
                    event: e.clone(),
                    encoded_bytes: serde_json::to_vec(e).unwrap().len(),
                })
            })
            .transpose()
    }
}
#[test]
fn native_bundle_engine_is_available_and_paging_preserves_gaps() {
    let state = StorageState::new(StorageLimits::default()).unwrap();
    let read = Reader {
        image: JournalImage::from_metadata(StorageMetadata::from_state(&state)).unwrap(),
        fence: Default::default(),
    };
    let work = WorkImage::from_ledger(state.work.clone(), state.retry_epochs).unwrap();
    assert!(prepare_native_bundle(&read, &work, &bundle("a", "test", true)).is_ok());
    assert_eq!(
        journal_page(&read, "test", None, 10, 1024)
            .unwrap()
            .cursor
            .position,
        0
    );
}

impl StorageState {
    fn reference_bundle(&mut self, b: &Bundle) -> Result<Vec<String>> {
        self.check_retry_epoch(
            b.receipt.retry_epoch,
            false,
            b.completed_work.as_ref().map(|(claim, now)| (claim, *now)),
        )?;
        if b.reactions
            .iter()
            .any(|work| work.cause.retry_epoch != b.receipt.retry_epoch)
        {
            return Err(Error::Conflict);
        }
        let mut next = self.clone();
        if let Some((claim, now)) = &b.completed_work {
            next.work.apply(WorkUpdate::Finish {
                claim: claim.clone(),
                now: *now,
                outcome: WorkOutcome::Done,
            })?;
        }
        next.count_bundle(b)?;
        if !b.reactions.is_empty() {
            if !b.changed {
                return Err(Error::NotCommitted);
            }
            next.work.enqueue(
                b.reaction_limits.as_ref().ok_or(Error::NotCommitted)?,
                b.reactions.clone(),
            )?;
        }
        let removed = next.reference_journal(b)?;
        next.work.validate_retry_epochs(next.retry_epochs)?;
        *self = next;
        Ok(removed)
    }
    fn reference_page(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<JournalPage> {
        if max_rows == 0 || max_bytes == 0 {
            return Err(Error::TooLarge);
        }
        let position = after.map_or(0, |c| c.position);
        if position < self.floor
            || position > self.head
            || after.is_some_and(|c| c.kind != kind || c.generation != self.generation)
        {
            return Err(Error::HistoryGap);
        }
        let mut cursor = JournalCursor {
            generation: self.generation.clone(),
            kind: kind.into(),
            position,
        };
        let mut events = vec![];
        let mut bytes = 0usize;
        for e in self.events.iter().filter(|e| e.position > position) {
            if e.row.key.kind == kind {
                let next = bytes
                    .checked_add(serde_json::to_vec(e).map_err(|_| Error::Storage)?.len())
                    .ok_or(Error::TooLarge)?;
                if next > max_bytes {
                    if events.is_empty() {
                        return Err(Error::TooLarge);
                    }
                    break;
                }
                if events.len() == max_rows {
                    break;
                }
                bytes = next;
                events.push(e.clone());
            }
            cursor.position = e.position;
        }
        Ok(JournalPage { events, cursor })
    }
}

#[test]
fn native_retention_and_canonical_delegation_match_prechange_oracle() {
    for rows in [1, 2, 9] {
        for bytes in [180, 400, 2000] {
            let mut state = StorageState::new(StorageLimits {
                journal_rows: rows,
                journal_bytes: bytes,
                ..Default::default()
            })
            .unwrap();
            let mut expected = state.clone();
            for (n, changed) in [true, false, true, true, true, false, true]
                .into_iter()
                .enumerate()
            {
                let b = bundle(
                    &format!("event-{n}"),
                    if n % 2 == 0 { "a" } else { "b" },
                    changed,
                );
                let want = expected.reference_bundle(&b);
                assert_eq!(state.bundle(&b), want);
                assert_eq!(
                    serde_json::to_value(&state).unwrap(),
                    serde_json::to_value(&expected).unwrap()
                );
            }
            let cursor = JournalCursor {
                position: state.floor,
                ..state.journal_head("a")
            };
            for max_rows in [0, 1, 2, 9] {
                for max_bytes in [0, 1, 180, 400, 2000] {
                    assert_eq!(
                        state.journal("a", Some(&cursor), max_rows, max_bytes),
                        expected.reference_page("a", Some(&cursor), max_rows, max_bytes)
                    );
                }
            }
        }
    }
}
#[test]
fn native_delta_matches_every_canonical_field_and_retired_identity() {
    let mut expected = StorageState::new(StorageLimits {
        journal_rows: 2,
        ..Default::default()
    })
    .unwrap();
    for n in 0..8 {
        let b = bundle(&format!("e-{n}"), "a", n % 3 != 1);
        let mut journal =
            JournalImage::from_metadata(StorageMetadata::from_state(&expected)).unwrap();
        let mut work =
            WorkImage::from_ledger(expected.work.clone(), expected.retry_epochs).unwrap();
        let delta = prepare_native_bundle(&journal, &work, &b).unwrap();
        let want = expected.reference_bundle(&b).unwrap();
        assert_eq!(
            delta
                .journal()
                .retired()
                .iter()
                .map(|e| e.event.identity.clone())
                .collect::<Vec<_>>(),
            want
        );
        delta.validate(&journal, &work).unwrap();
        let (j, w) = delta.into_parts();
        journal.apply(j).unwrap();
        work.apply(w).unwrap();
        let actual = journal
            .into_metadata()
            .reassemble(work.canonical(), expected.operator.clone());
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
    }
}

#[test]
fn exact_byte_thresholds_max_positions_and_permissive_row_contract_match_oracle() {
    let b = bundle("e", "a", true);
    let event = JournalEvent {
        position: 1,
        identity: b.receipt.identity.clone(),
        row: b.receipt.row.clone(),
    };
    let size = serde_json::to_vec(&event).unwrap().len();
    for limit in [size - 1, size, size + 1] {
        let mut actual = StorageState::new(StorageLimits {
            journal_bytes: limit,
            ..Default::default()
        })
        .unwrap();
        let mut expected = actual.clone();
        assert_eq!(actual.bundle(&b), expected.reference_bundle(&b));
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        if limit < size {
            assert_eq!(actual.bundle(&b), Err(Error::TooLarge));
        } else {
            assert_eq!(actual.journal("a", None, 1, size).unwrap().events.len(), 1);
            assert_eq!(actual.journal("a", None, 1, size - 1), Err(Error::TooLarge));
        }
    }
    let mut s = StorageState::new(StorageLimits::default()).unwrap();
    s.head = u64::MAX;
    s.floor = u64::MAX;
    let mut expected = s.clone();
    assert_eq!(s.bundle(&b), expected.reference_bundle(&b));
    assert_eq!(s.bundle(&b), Err(Error::TooLarge));
    let mut bad = bundle("odd", "", true);
    bad.receipt.row.key.id = "".into();
    bad.receipt.row.revision = 0;
    let mut s = StorageState::new(StorageLimits::default()).unwrap();
    let mut expected = s.clone();
    assert_eq!(s.bundle(&bad), expected.reference_bundle(&bad));
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
}
#[test]
fn paging_progress_across_other_kinds_and_mismatched_cursors_matches_oracle() {
    let mut s = StorageState::new(StorageLimits::default()).unwrap();
    for (id, kind) in [("1", "b"), ("2", "a"), ("3", "b"), ("4", "a"), ("5", "b")] {
        s.reference_bundle(&bundle(id, kind, true)).unwrap();
    }
    let image = JournalImage::from_metadata(StorageMetadata::from_state(&s)).unwrap();
    let page = journal_page(&image, "a", None, 1, 10000).unwrap();
    assert_eq!(page.cursor.position, 3);
    assert_eq!(page.events[0].position, 2);
    for cursor in [
        None,
        Some(JournalCursor {
            position: 6,
            ..s.journal_head("a")
        }),
        Some(s.journal_head("b")),
        Some(JournalCursor {
            generation: "different".into(),
            ..s.journal_head("a")
        }),
    ] {
        for rows in [0, 1, 10] {
            for bytes in [0, 1, 10000] {
                assert_eq!(
                    journal_page(&image, "a", cursor.as_ref(), rows, bytes),
                    s.reference_page("a", cursor.as_ref(), rows, bytes)
                );
            }
        }
    }
}
fn claimed() -> (StorageState, ClaimKey) {
    let mut s = StorageState::new(StorageLimits::default()).unwrap();
    s.work
        .enqueue(
            &ReactionLimits::default(),
            vec![PendingWork {
                id: "source".into(),
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
                payload: WorkPayload::Source(bundle("src", "a", true).receipt.row),
            }],
        )
        .unwrap();
    let claim = match s.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(c) => c.key(),
        other => panic!("expected claim: {other:?}"),
    };
    (s, claim)
}
#[test]
fn completion_child_enqueue_replay_and_error_precedence_match_frozen_bundle() {
    for case in 0..7 {
        let (mut actual, claim) = claimed();
        let mut b = bundle("result", "a", true);
        b.completed_work = Some((claim.clone(), 11));
        let mut child = actual.work.records()[0].pending.clone();
        child.id = "child".into();
        child.cause.parent = Some("source".into());
        child.cause.depth = 1;
        b.reactions = vec![child];
        b.reaction_limits = Some(ReactionLimits::default());
        match case {
            1 => b.changed = false,
            2 => {
                b.completed_work.as_mut().unwrap().0.generation += 1;
                actual.limits.receipts = 1;
                actual.receipts = 1;
            }
            3 => {
                actual.limits.receipts = 1;
                actual.receipts = 1;
            }
            4 => b.reaction_limits = None,
            5 => b.reactions[0].cause.retry_epoch = 1,
            6 => b.receipt.retry_epoch = 1,
            _ => (),
        }
        let mut expected = actual.clone();
        let want = expected.reference_bundle(&b);
        let header = JournalImage::from_metadata(StorageMetadata::from_state(&actual)).unwrap();
        let mut work = WorkImage::from_ledger(actual.work.clone(), actual.retry_epochs).unwrap();
        assert_eq!(
            header.header().check_retry_epoch(
                0,
                true,
                b.completed_work.as_ref().map(|(c, n)| (c, *n)),
                &work
            ),
            Ok(())
        );
        let prepared = prepare_native_bundle(&header, &work, &b);
        match (prepared, &want) {
            (Ok(d), Ok(ids)) => {
                assert_eq!(
                    d.journal()
                        .retired()
                        .iter()
                        .map(|e| e.event.identity.clone())
                        .collect::<Vec<_>>(),
                    *ids
                );
                let (j, w) = d.into_parts();
                let mut image = header;
                image.apply(j).unwrap();
                work.apply(w).unwrap();
                assert_eq!(
                    serde_json::to_value(
                        image
                            .into_metadata()
                            .reassemble(work.canonical(), actual.operator.clone())
                    )
                    .unwrap(),
                    serde_json::to_value(&expected).unwrap()
                );
            }
            (Err(e), Err(want)) => assert_eq!(&e, want),
            _ => panic!("native/canonical outcome disagrees"),
        }
        assert_eq!(actual.bundle(&b), want);
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}

#[test]
fn one_large_append_retires_multiple_exact_prefix_entries() {
    let mut expected = StorageState::new(StorageLimits {
        journal_bytes: 1000,
        ..Default::default()
    })
    .unwrap();
    for n in 0..5 {
        expected
            .reference_bundle(&bundle(&format!("old{n}"), "a", true))
            .unwrap();
    }
    let mut b = bundle("large", "a", true);
    b.receipt.row.value = Some(json!({"text":"x".repeat(600)}));
    let mut journal = JournalImage::from_metadata(StorageMetadata::from_state(&expected)).unwrap();
    let work = WorkImage::from_ledger(expected.work.clone(), expected.retry_epochs).unwrap();
    let d = prepare_native_bundle(&journal, &work, &b).unwrap();
    let retired = expected.reference_bundle(&b).unwrap();
    assert!(retired.len() >= 3);
    assert_eq!(
        d.journal()
            .retired()
            .iter()
            .map(|e| e.event.identity.clone())
            .collect::<Vec<_>>(),
        retired
    );
    assert_eq!(d.journal().entry_preconditions().count(), retired.len() + 1);
    journal.apply(d.into_parts().0).unwrap();
    assert_eq!(
        serde_json::to_value(journal.into_metadata()).unwrap(),
        serde_json::to_value(StorageMetadata::from_state(&expected)).unwrap()
    );
}
