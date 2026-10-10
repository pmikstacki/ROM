use super::*;
fn parts() -> MetadataHeaderParts {
    MetadataHeaderParts {
        retry_epochs: RetryEpochs::default(),
        limits: StorageLimits::default(),
        generation: "journal".into(),
        head: 0,
        floor: 0,
        receipts: 0,
        effects: 0,
        journal_records: 0,
        journal_bytes: 0,
    }
}
#[test]
fn scalar_header_round_trip() {
    let p = parts();
    let h = MetadataHeader::from_parts(p.clone()).unwrap();
    assert_eq!(h.parts(), p);
    assert_eq!(h.retry_epochs(), p.retry_epochs);
}
#[test]
fn rejects_invalid_generation_epochs_and_limits() {
    let mut cases = vec![];
    for generation in [String::new(), "x".repeat(1024)] {
        let mut p = parts();
        p.generation = generation;
        cases.push(p);
    }
    let mut p = parts();
    p.retry_epochs.admission_floor = 1;
    cases.push(p);
    let mut p = parts();
    p.retry_epochs.replay_floor = 1;
    cases.push(p);
    for index in 0..4 {
        let mut p = parts();
        match index {
            0 => p.limits.receipts = 0,
            1 => p.limits.effects = 0,
            2 => p.limits.journal_rows = 0,
            _ => p.limits.journal_bytes = 0,
        };
        cases.push(p);
    }
    for p in cases {
        assert!(MetadataHeader::from_parts(p).is_err());
    }
}
#[test]
fn rejects_disagreement_capacity_and_extreme_counts() {
    let mut cases = vec![];
    let mut p = parts();
    p.floor = 1;
    cases.push(p);
    let mut p = parts();
    p.head = 1;
    cases.push(p);
    let mut p = parts();
    p.journal_records = usize::MAX;
    cases.push(p);
    let mut p = parts();
    p.journal_bytes = usize::MAX;
    cases.push(p);
    let mut p = parts();
    p.receipts = p.limits.receipts + 1;
    cases.push(p);
    let mut p = parts();
    p.effects = p.limits.effects + 1;
    cases.push(p);
    let mut p = parts();
    p.head = 1;
    p.journal_records = 1;
    cases.push(p);
    for p in cases {
        assert!(MetadataHeader::from_parts(p).is_err());
    }
}
