//! Atomic-update admission and downstream default compatibility.
use super::*;
use crate::storage_support::work::validate_work_update_batch;
use std::sync::Mutex;

struct Legacy(Mutex<WorkLedger>);
impl Storage for Legacy {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: false,
            snapshots: false,
            effects: false,
        }
    }
    fn load(&self, _: &Key) -> Result<Option<Row>> {
        unreachable!()
    }
    fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        unreachable!()
    }
    fn receipt(&self, _: &str) -> Result<Option<Receipt>> {
        unreachable!()
    }
    fn commit(&self, _: &Bundle) -> Result<Receipt> {
        unreachable!()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.0.lock().map_err(|_| Error::Panicked)?.apply(update)
    }
}
fn ledger() -> WorkLedger {
    let mut ledger = WorkLedger::default();
    ledger
        .enqueue(
            &ReactionLimits::default(),
            vec![PendingWork {
                id: "a".into(),
                cause: Cause {
                    retry_epoch: 0,
                    root: "root".into(),
                    parent: None,
                    depth: 0,
                    started_at: 10,
                    path: vec![],
                },
                definition: "mapper".into(),
                version: 1,
                service_key: "service".into(),
                delivery_profile: DeliveryProfile::AtLeastOnce,
                not_before: None,
                payload: WorkPayload::Source(Row {
                    key: Key {
                        kind: "Source".into(),
                        id: "a".into(),
                    },
                    revision: 1,
                    value: Some(json!({})),
                    protected: ProtectedMetadata::default(),
                }),
            }],
        )
        .unwrap();
    ledger
}
fn state(storage: &Legacy) -> Value {
    serde_json::to_value(&*storage.0.lock().unwrap()).unwrap()
}
#[test]
fn atomic_default_empty_does_not_claim_work() {
    let storage = Legacy(Mutex::new(ledger()));
    let before = state(&storage);
    assert_eq!(storage.reaction_updates_atomic(vec![]), Ok(vec![]));
    assert_eq!(state(&storage), before);
}
#[test]
fn atomic_default_singleton_preserves_claim_and_error() {
    let storage = Legacy(Mutex::new(ledger()));
    let results = storage
        .reaction_updates_atomic(vec![WorkUpdate::Claim { now: 10 }])
        .unwrap();
    let WorkResult::Claimed(claim) = &results[0] else {
        panic!("expected durable claim")
    };
    assert_eq!(storage.0.lock().unwrap().records()[0], claim.work);
    let before = state(&storage);
    assert_eq!(
        storage.reaction_updates_atomic(vec![WorkUpdate::Finish {
            claim: ClaimKey {
                id: "missing".into(),
                generation: 1
            },
            now: 11,
            outcome: WorkOutcome::Done,
        }]),
        Err(Error::Missing)
    );
    assert_eq!(state(&storage), before);
}
#[test]
fn atomic_default_multi_update_is_unsupported_before_first_claim() {
    let storage = Legacy(Mutex::new(ledger()));
    let before = state(&storage);
    assert!(matches!(
        storage.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 10 }; 2]),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(state(&storage), before);
}
#[test]
fn atomic_default_rejects_oversized_operation_count_before_mutation() {
    let storage = Legacy(Mutex::new(ledger()));
    let before = state(&storage);
    assert_eq!(
        storage.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 10 }; 33]),
        Err(Error::TooLarge)
    );
    assert_eq!(state(&storage), before);
}
#[test]
fn atomic_batch_byte_admission_charges_whole_json_at_exact_boundary() {
    // Hand-counted [{"Claim":{"now":10}}] is 22 UTF-8 bytes.
    let updates = vec![WorkUpdate::Claim { now: 10 }];
    assert_eq!(validate_work_update_batch(&updates, 22), Ok(22));
    assert_eq!(
        validate_work_update_batch(&updates, 21),
        Err(Error::TooLarge)
    );
    assert_eq!(validate_work_update_batch(&[], 2), Ok(2));
    assert_eq!(validate_work_update_batch(&[], 1), Err(Error::TooLarge));
    // 32 twenty-byte objects, 31 commas and two array delimiters.
    assert_eq!(
        validate_work_update_batch(&vec![WorkUpdate::Claim { now: 10 }; 32], 673),
        Ok(673)
    );
    assert_eq!(
        validate_work_update_batch(&vec![WorkUpdate::Claim { now: 10 }; 33], usize::MAX),
        Err(Error::TooLarge)
    );
}
#[test]
fn atomic_batch_byte_admission_counts_multibyte_and_escape_bytes() {
    let updates = vec![WorkUpdate::Finish {
        claim: ClaimKey {
            id: "雪\"".into(),
            generation: 1,
        },
        now: 10,
        outcome: WorkOutcome::Done,
    }];
    let bytes = serde_json::to_vec(&updates).unwrap().len();
    assert_eq!(validate_work_update_batch(&updates, bytes), Ok(bytes));
    assert_eq!(
        validate_work_update_batch(&updates, bytes - 1),
        Err(Error::TooLarge)
    );
}
#[test]
fn singleton_fallback_retains_prior_completion_after_later_failure() {
    let storage = Legacy(Mutex::new(ledger()));
    let WorkResult::Claimed(claim) = storage
        .reaction_update(WorkUpdate::Claim { now: 10 })
        .unwrap()
    else {
        panic!("claim")
    };
    storage
        .reaction_updates_atomic(vec![WorkUpdate::Materialize {
            claim: claim.key(),
            now: 11,
            children: vec![],
        }])
        .unwrap();
    assert_eq!(
        storage.reaction_updates_atomic(vec![WorkUpdate::Finish {
            claim: ClaimKey {
                id: "missing".into(),
                generation: 1
            },
            now: 11,
            outcome: WorkOutcome::Done,
        }]),
        Err(Error::Missing)
    );
    assert_eq!(
        storage.0.lock().unwrap().records()[0].state,
        WorkState::Done
    );
}
#[test]
fn singleton_updates_match_sequential_image_and_reject_old_fence() {
    use super::incremental::{WorkRead, prepare_update};
    use super::incremental_bridge::WorkImage;
    let storage = Legacy(Mutex::new(ledger()));
    let mut image = WorkImage::from_ledger(ledger(), RetryEpochs::default()).unwrap();
    let stale = prepare_update(&image, WorkUpdate::Claim { now: 10 }).unwrap();
    let update = WorkUpdate::Claim { now: 10 };
    let expected = image
        .apply(prepare_update(&image, update.clone()).unwrap())
        .unwrap();
    assert_eq!(
        storage.reaction_updates_atomic(vec![update]).unwrap(),
        vec![expected.clone()]
    );
    let before = serde_json::to_value(image.canonical()).unwrap();
    assert_eq!(image.apply(stale), Err(Error::Conflict));
    assert_eq!(serde_json::to_value(image.canonical()).unwrap(), before);
    let WorkResult::Claimed(claim) = expected else {
        panic!("claim")
    };
    let update = WorkUpdate::Materialize {
        claim: claim.key(),
        now: 11,
        children: vec![],
    };
    let expected = image
        .apply(prepare_update(&image, update.clone()).unwrap())
        .unwrap();
    assert_eq!(
        storage.reaction_updates_atomic(vec![update]).unwrap(),
        vec![expected]
    );
    assert_eq!(
        state(&storage),
        serde_json::to_value(image.canonical()).unwrap()
    );
    assert!(image.next_candidate(100, None).unwrap().is_none());
}
