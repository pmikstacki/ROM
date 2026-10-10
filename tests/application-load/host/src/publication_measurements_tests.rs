use super::*;
use rom_sqlite::{
    PublicationCategory, PublicationMeasurement, PublicationSnapshot, StageOperation,
};

fn maximum_snapshot() -> PublicationSnapshot {
    let categories = [
        PublicationCategory::Metadata,
        PublicationCategory::EventPayload,
        PublicationCategory::WorkHeader,
        PublicationCategory::WorkRecord,
        PublicationCategory::WorkRoot,
    ];
    PublicationSnapshot {
        entries: std::array::from_fn(|index| PublicationMeasurement {
            operation: if index < 5 {
                StageOperation::Commit
            } else {
                StageOperation::WorkUpdate
            },
            category: categories[index % 5],
            samples: u64::MAX,
            encoded_bytes: u64::MAX,
            dropped_samples: u64::MAX,
            saturated: true,
        }),
        poison_recovered: true,
    }
}

#[test]
fn complete_publication_evidence_fits_existing_64_kib_limit() {
    let mut batches = Vec::new();
    for rows in (100..=10000).step_by(100) {
        record(&mut batches, rows, maximum_snapshot()).unwrap();
    }
    let proof = proof(maximum_snapshot(), &batches, maximum_snapshot());
    assert!(serde_json::to_vec(&proof).unwrap().len() <= 65536);
    assert_eq!(
        proof["baseline"]["entries"][0]["encoded_bytes"],
        serde_json::json!(u64::MAX)
    );
    assert_eq!(proof["final"]["poison_recovered"], true);
    assert_eq!(proof["batches"][99]["completed_rows"], 10000);
    assert_eq!(
        proof["batches"][99]["cumulative_samples_and_encoded_bytes"][9][1],
        serde_json::json!(u64::MAX)
    );
    assert_eq!(proof["durable_commit_bytes"], false);
    assert_eq!(proof["batch_comparisons_usable"], false);
}

fn clean_snapshot() -> PublicationSnapshot {
    let mut snapshot = maximum_snapshot();
    snapshot.poison_recovered = false;
    for entry in &mut snapshot.entries {
        entry.samples = 0;
        entry.encoded_bytes = 0;
        entry.dropped_samples = 0;
        entry.saturated = false;
    }
    snapshot
}

#[test]
fn either_boundary_with_poison_drops_or_saturation_disables_batch_comparison() {
    assert_eq!(
        proof(clean_snapshot(), &[], clean_snapshot())["batch_comparisons_usable"],
        true
    );
    for final_boundary in [false, true] {
        for fault in 0..3 {
            let mut baseline = clean_snapshot();
            let mut final_snapshot = clean_snapshot();
            let snapshot = if final_boundary {
                &mut final_snapshot
            } else {
                &mut baseline
            };
            match fault {
                0 => snapshot.poison_recovered = true,
                1 => snapshot.entries[0].dropped_samples = 1,
                _ => snapshot.entries[0].saturated = true,
            }
            assert_eq!(
                proof(baseline, &[], final_snapshot)["batch_comparisons_usable"],
                false
            );
        }
    }
}

#[test]
fn a_batch_overflow_cannot_replace_prior_completed_batch_evidence() {
    let mut batches = Vec::new();
    for rows in (100..=10000).step_by(100) {
        record(&mut batches, rows, maximum_snapshot()).unwrap();
    }
    assert_eq!(
        record(&mut batches, 10100, maximum_snapshot()),
        Err(rom::Error::TooLarge)
    );
    assert_eq!(batches.len(), 100);
    assert_eq!(batches[99].completed_rows, 10000);
}
