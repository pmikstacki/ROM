//! Shared real-adapter probes. Call with an isolated, initialized reaction ledger.
use rom::{ClaimKey, Error, Storage, WorkOutcome, WorkResult, WorkState, WorkUpdate};

/// Requires at least two eligible items. A later invalid transition rolls back
/// the earlier transition in an atomic group; singleton calls retain a prefix.
pub fn assert_atomic_results_rollback_and_singleton_prefix(storage: &dyn Storage, now: u64) {
    let results = storage
        .reaction_updates_atomic(vec![WorkUpdate::Claim { now }; 2])
        .expect("adapter must support atomic ordered claim updates");
    let claims: Vec<_> = results
        .into_iter()
        .map(|result| match result {
            WorkResult::Claimed(claim) => claim,
            other => panic!("fixture requires two eligible items: {other:?}"),
        })
        .collect();
    assert_ne!(claims[0].key(), claims[1].key());
    let first = WorkUpdate::Finish {
        claim: claims[0].key(),
        now,
        outcome: WorkOutcome::Done,
    };
    let invalid = WorkUpdate::Finish {
        claim: ClaimKey {
            id: claims[1].key().id,
            generation: 0,
        },
        now,
        outcome: WorkOutcome::Done,
    };
    let before = storage.reaction_records().unwrap();
    assert_eq!(
        storage.reaction_updates_atomic(vec![first.clone(), invalid.clone()]),
        Err(Error::Conflict)
    );
    assert_eq!(storage.reaction_records().unwrap(), before);
    assert_eq!(
        storage.reaction_updates_atomic(vec![first]),
        Ok(vec![WorkResult::Changed])
    );
    assert_eq!(
        storage.reaction_updates_atomic(vec![invalid]),
        Err(Error::Conflict)
    );
    let after = storage.reaction_records().unwrap();
    assert_eq!(
        after
            .iter()
            .find(|record| record.pending.id == claims[0].key().id)
            .unwrap()
            .state,
        WorkState::Done
    );
    assert_eq!(
        after
            .iter()
            .find(|record| record.pending.id == claims[1].key().id)
            .unwrap(),
        before
            .iter()
            .find(|record| record.pending.id == claims[1].key().id)
            .unwrap()
    );
}

/// Repeated Claim updates must observe earlier transaction-local changes,
/// rather than applying several deltas prepared against one stale read fence.
/// Requires a fresh, complete, uncompacted ledger. This reconstructs root attempt
/// counts from all records; it is not an integrity proof for retained history.
pub fn assert_atomic_claims_match_sequential_image(
    storage: &dyn Storage,
    now: u64,
    limits: &rom::ReactionLimits,
) {
    use rom::storage_support::work::{WorkImage, prepare_update};
    let records = storage.reaction_records().unwrap();
    // WorkLedger's public import requires its complete canonical root projection.
    let mut roots = std::collections::BTreeMap::<String, u32>::new();
    for record in &records {
        let used = roots.entry(record.pending.cause.root.clone()).or_default();
        *used = used.checked_add(record.attempts).unwrap();
    }
    let ledger: rom::WorkLedger = serde_json::from_value(serde_json::json!({
        "limits": limits,
        "work": records.iter().map(|r| (r.pending.id.clone(), r.clone())).collect::<std::collections::BTreeMap<_,_>>(),
        "roots": roots,
    })).unwrap();
    let mut image = WorkImage::from_ledger(ledger, storage.retry_epochs().unwrap()).unwrap();
    let mut expected = vec![];
    for _ in 0..2 {
        expected.push(
            image
                .apply(prepare_update(&image, WorkUpdate::Claim { now }).unwrap())
                .unwrap(),
        );
    }
    assert_eq!(
        storage.reaction_updates_atomic(vec![WorkUpdate::Claim { now }; 2]),
        Ok(expected)
    );
    assert_eq!(
        storage.reaction_records().unwrap(),
        image.canonical().records()
    );
}
