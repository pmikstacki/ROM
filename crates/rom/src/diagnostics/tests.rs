//! Diagnostic boundary tests; expected tokens come from an independent HMAC calculation.
use super::*;
use crate::{Cause, ClaimKey};

fn channel(capacity: usize) -> (DiagnosticSink, DiagnosticReader) {
    Diagnostics::bounded(
        DiagnosticOptions::new(DiagnosticKey::new([23; 32]), [1; 16]).capacity(capacity),
    )
    .unwrap()
}
#[test]
fn bounded_output_drops_without_waiting_and_keeps_fixed_categories() {
    let (sink, mut reader) = channel(1);
    let mut operation = sink.operation("secret", None, None, 0);
    operation.stage(
        DiagnosticStage::Commit,
        DiagnosticOutcome::Unknown,
        Some(5),
        0,
    );
    operation.stage(
        DiagnosticStage::Receipt,
        DiagnosticOutcome::Replay,
        Some(8),
        0,
    );
    operation.publish();
    assert_eq!(reader.stats().enqueued, 1);
    assert_eq!(reader.stats().dropped_full, 1);
    let event = reader.try_recv().unwrap();
    assert_eq!(event.stage, DiagnosticStage::Commit);
    assert_eq!(event.outcome, DiagnosticOutcome::Unknown);
    assert_eq!(event.elapsed_ns, Some(5));
    assert!(reader.try_recv().is_none());
    assert!(!serde_json::to_string(&event).unwrap().contains("secret"));
}
#[test]
fn collector_drop_does_not_publish_and_explicit_publication_has_a_finite_budget() {
    let (sink, mut reader) = channel(64);
    let mut abandoned = sink.operation("secret", None, None, 0);
    abandoned.stage(
        DiagnosticStage::Execution,
        DiagnosticOutcome::Started,
        None,
        0,
    );
    drop(abandoned);
    assert!(reader.try_recv().is_none());
    let mut operation = sink.operation("secret", None, None, 0);
    for _ in 0..40 {
        operation.stage(
            DiagnosticStage::Execution,
            DiagnosticOutcome::Started,
            None,
            0,
        );
    }
    operation.publish();
    let mut count = 0;
    while reader.try_recv().is_some() {
        count += 1;
    }
    assert_eq!(count, 32);
    assert_eq!(reader.stats().dropped_collector, 8);
}
#[test]
fn closing_reader_disables_later_records_without_retaining_core_handles() {
    let (sink, mut reader) = channel(8);
    reader.close();
    let mut operation = sink.operation("secret", None, None, 0);
    operation.stage(
        DiagnosticStage::Commit,
        DiagnosticOutcome::Succeeded,
        None,
        1,
    );
    operation.publish();
    assert_eq!(reader.stats().dropped_closed, 1);
    assert!(reader.try_recv().is_none());
}
#[test]
fn generation_fences_claim_owner_without_changing_restart_root_or_work() {
    let (sink, mut reader) = channel(8);
    let cause = Cause {
        retry_epoch: 0,
        root: "root secret".into(),
        parent: Some("parent secret".into()),
        depth: 2,
        started_at: 0,
        path: vec!["a".into(), "b".into()],
    };
    let mut first = sink.operation(
        "operation secret",
        Some(&cause),
        Some(&ClaimKey {
            id: "work secret".into(),
            generation: 1,
        }),
        1,
    );
    let mut second = sink.operation(
        "operation secret",
        Some(&cause),
        Some(&ClaimKey {
            id: "work secret".into(),
            generation: 2,
        }),
        2,
    );
    first.stage(DiagnosticStage::Work, DiagnosticOutcome::Started, None, 0);
    second.stage(DiagnosticStage::Work, DiagnosticOutcome::Started, None, 0);
    first.publish();
    second.publish();
    let a = reader.try_recv().unwrap();
    let b = reader.try_recv().unwrap();
    assert_eq!(a.root, b.root);
    assert_eq!(a.parent, b.parent);
    assert_eq!(a.work, b.work);
    assert_ne!(a.claim, b.claim);
    assert_eq!(a.depth, 2);
    assert_eq!(b.attempt, 2);
}

fn token(hex: &str) -> DiagnosticToken {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
    }
    DiagnosticToken(bytes)
}
#[test]
fn independent_hmac_vectors_pin_versioned_identity_domains() {
    use super::correlation::{Correlation, Domain};
    let correlation = Correlation::new(DiagnosticKey::new([23; 32])).unwrap();
    // Fixture computed independently with Node crypto HMAC, not through this implementation.
    for (domain, expected) in [
        (
            Domain::Operation,
            "82cde10b521bc1fba6d1601921d1174df83cc4a7eb44c7f75f3dc50cbe982f23",
        ),
        (
            Domain::Root,
            "f3783efa470a1547dce979110fe88766f15fbdf757f4c87f101d1b065ed71d4e",
        ),
        (
            Domain::Work,
            "e8767ab608289cdcbcfca03867dd12385151eaa45e635f21350f543bdd91c831",
        ),
    ] {
        assert_eq!(
            correlation.token(domain, &[b"operation secret"]),
            token(expected)
        );
    }
    assert_eq!(
        correlation.token(Domain::Claim, &[b"work secret", &1_u64.to_be_bytes()]),
        token("240434ea393eafc730fe56620aa778d57f6f0a37363a2f325b71c5f62b661818"),
    );
    assert_ne!(
        correlation.token(Domain::Work, &[b"ab", b"c"]),
        correlation.token(Domain::Work, &[b"a", b"bc"]),
    );
}
#[test]
fn parent_reference_matches_the_observed_parent_work_namespace() {
    let (sink, mut reader) = channel(8);
    let parent_key = ClaimKey {
        id: "parent work".into(),
        generation: 1,
    };
    let mut parent = sink.operation("parent receipt", None, Some(&parent_key), 1);
    let child_cause = Cause {
        retry_epoch: 0,
        root: "root".into(),
        parent: Some("parent work".into()),
        depth: 2,
        started_at: 0,
        path: vec!["copy".into(), "0".into()],
    };
    let mut child = sink.operation("child receipt", Some(&child_cause), None, 0);
    parent.stage(
        DiagnosticStage::Reaction,
        DiagnosticOutcome::Succeeded,
        None,
        0,
    );
    child.stage(
        DiagnosticStage::Commit,
        DiagnosticOutcome::Succeeded,
        None,
        1,
    );
    parent.publish();
    child.publish();
    let parent = reader.try_recv().unwrap();
    let child = reader.try_recv().unwrap();
    assert!(parent.work.is_some());
    assert_eq!(child.parent, parent.work);
}

#[test]
fn concurrent_publication_has_unique_allocations_without_assuming_receive_order() {
    let (sink, mut reader) = channel(512);
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let sink = sink.clone();
            std::thread::spawn(move || {
                for _ in 0..100 {
                    let mut operation = sink.operation("receipt", None, None, 0);
                    operation.stage(
                        DiagnosticStage::Commit,
                        DiagnosticOutcome::Succeeded,
                        None,
                        1,
                    );
                    operation.publish();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let mut sequences = Vec::new();
    while let Some(event) = reader.try_recv() {
        sequences.push(event.sequence);
    }
    sequences.sort_unstable();
    assert_eq!(sequences, (0..400).collect::<Vec<_>>());
    assert_eq!(reader.stats().enqueued, 400);
    assert_eq!(reader.stats().dropped_full, 0);
}
#[test]
fn capacity_loss_leaves_an_allocation_gap_instead_of_false_continuity() {
    let (sink, mut reader) = channel(1);
    let mut operation = sink.operation("receipt", None, None, 0);
    for _ in 0..2 {
        operation.stage(
            DiagnosticStage::Commit,
            DiagnosticOutcome::Succeeded,
            None,
            1,
        );
    }
    operation.publish();
    assert_eq!(reader.try_recv().unwrap().sequence, 0);
    let mut next = sink.operation("receipt", None, None, 0);
    next.stage(DiagnosticStage::Receipt, DiagnosticOutcome::Replay, None, 0);
    next.publish();
    assert_eq!(reader.try_recv().unwrap().sequence, 2);
    assert_eq!(reader.stats().dropped_full, 1);
}
