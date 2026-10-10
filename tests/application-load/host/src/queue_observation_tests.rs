use super::*;
use std::time::Duration;
#[test]
fn confirmed_commit_then_claim_has_local_interval_without_reset_on_replay() {
    let mut q = QueueObservation::new(2);
    let now = Instant::now();
    q.offer("a").unwrap();
    q.committed("a", now);
    q.claimed("a", now + Duration::from_millis(3));
    assert_eq!(q.samples(), &[3_000_000]);
    q.offer("a").unwrap();
    q.committed("a", now);
    q.claimed("a", now + Duration::from_millis(4));
    assert_eq!(q.samples().len(), 1);
    assert_eq!(q.missing(), 1);
}
#[test]
fn claim_racing_confirmation_is_unavailable_not_negative_or_forged() {
    let mut q = QueueObservation::new(1);
    let now = Instant::now();
    q.offer("a").unwrap();
    q.claimed("a", now);
    q.committed("a", now + Duration::from_millis(1));
    assert!(q.samples().is_empty());
    assert_eq!(q.missing(), 1);
}
#[test]
fn failed_commit_unknown_claim_and_capacity_are_explicit() {
    let mut q = QueueObservation::new(1);
    let now = Instant::now();
    q.offer("a").unwrap();
    q.failed("a");
    q.claimed("a", now);
    q.claimed("previous-process", now);
    assert_eq!(q.missing(), 2);
    assert!(matches!(q.offer("b"), Err(rom::Error::TooLarge)));
    assert!(q.samples().is_empty());
}
#[test]
fn duplicate_confirmation_cannot_move_original_start_forward() {
    let mut q = QueueObservation::new(1);
    let now = Instant::now();
    q.offer("a").unwrap();
    q.committed("a", now);
    q.offer("a").unwrap();
    q.committed("a", now + Duration::from_millis(5));
    q.claimed("a", now + Duration::from_millis(8));
    assert_eq!(q.samples(), &[8_000_000]);
}
#[test]
fn maximum_timing_array_fits_fixed_output_budget() {
    let samples = vec![u64::MAX; 12000];
    let record = serde_json::json!({"semantics":"fixture-commit-confirmation-to-claim-confirmation","restart_durable":false,"nanosecond_samples":samples,"missing":12000,"rejected":12000});
    assert!(serde_json::to_vec(&record).unwrap().len() <= 256 * 1024);
}
