use super::*;
use crate::diagnostics::{DiagnosticOutcome, DiagnosticStage};

#[test]
fn sequence_exhaustion_drops_instead_of_reusing_an_observed_sequence() {
    let (sink, mut reader) = Diagnostics::bounded(DiagnosticOptions::new(
        DiagnosticKey::new([23; 32]),
        [1; 16],
    ))
    .unwrap();
    sink.inner.sequence.store(u64::MAX, Ordering::Relaxed);
    let mut record = sink.operation("receipt", None, None, 0);
    record.stage(
        DiagnosticStage::Commit,
        DiagnosticOutcome::Succeeded,
        None,
        1,
    );
    record.publish();
    assert!(reader.try_recv().is_none());
    assert_eq!(reader.stats().dropped_sequence, 1);
    assert_eq!(reader.stats().enqueued, 0);
}
#[test]
fn loss_count_saturates_instead_of_resetting_on_overflow() {
    let counter = AtomicU64::new(u64::MAX - 1);
    increment(&counter);
    increment(&counter);
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
}
