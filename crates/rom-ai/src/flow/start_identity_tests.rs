//! Source-only proposed tests. ROOT owns execution admission.
use super::start_identity::CallbackIdentity;
use std::sync::atomic::AtomicU64;
#[test]
fn duplicate_delivery_invocations_have_distinct_start_commands() {
    let counter = AtomicU64::new(1);
    let a = CallbackIdentity::allocate("work-a", 2, &counter).unwrap();
    let b = CallbackIdentity::allocate("work-a", 2, &counter).unwrap();
    let first = a.start("run", "run:2:2", 9).unwrap();
    assert_eq!(first, a.start("run", "run:2:2", 9).unwrap());
    assert_ne!(first, b.start("run", "run:2:2", 9).unwrap());
}
#[test]
fn nonce_exhaustion_is_closed_without_wraparound() {
    let counter = AtomicU64::new(u64::MAX);
    assert!(CallbackIdentity::allocate("work", 1, &counter).is_err());
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), u64::MAX);
}
#[test]
fn restart_and_operator_claims_keep_durable_identity_distinct() {
    let a = CallbackIdentity::allocate("work", 1, &AtomicU64::new(1)).unwrap();
    let b = CallbackIdentity::allocate("work", 2, &AtomicU64::new(1)).unwrap();
    assert_ne!(
        a.start("run", "run:2:2", 9).unwrap(),
        b.start("run", "run:2:2", 9).unwrap()
    );
    assert_ne!(
        a.start("a:b", "c", 9).unwrap(),
        a.start("a", "b:c", 9).unwrap()
    );
}
