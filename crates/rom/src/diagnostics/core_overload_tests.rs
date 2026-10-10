//! Counter saturation and concurrent-total controls for the staged RED skeleton.
use super::core_overload::CoreOverloadCounters;
use std::sync::{Arc, atomic::Ordering};

#[test]
fn all_four_overload_counters_saturate_without_wrapping_and_mark_lost_event() {
    for category in 0..4 {
        let counts = CoreOverloadCounters::default();
        let counter = match category {
            0 => &counts.action_no_permits,
            1 => &counts.io_no_permits,
            2 => &counts.observation_generation_exhausted,
            _ => &counts.actor_generation_exhausted,
        };
        counter.store(u64::MAX - 1, Ordering::Relaxed);
        let increment = || match category {
            0 => counts.record_action_no_permits(),
            1 => counts.record_io_no_permits(),
            2 => counts.record_observation_generation_exhausted(),
            _ => counts.record_actor_generation_exhausted(),
        };
        increment();
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert!(!counts.snapshot().overflowed);
        increment();
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert!(counts.snapshot().overflowed);
    }
}
#[test]
fn concurrent_boundary_increments_are_exact_after_join_and_instance_local() {
    let counts = Arc::new(CoreOverloadCounters::default());
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let counts = counts.clone();
            std::thread::spawn(move || {
                for _ in 0..64 {
                    counts.record_io_no_permits();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let snapshot = counts.snapshot();
    assert_eq!(snapshot.io_no_permits, 256);
    assert_eq!(snapshot.action_no_permits, 0);
    assert_eq!(snapshot.observation_generation_exhausted, 0);
    assert_eq!(snapshot.actor_generation_exhausted, 0);
    assert!(!snapshot.overflowed);
    assert_eq!(CoreOverloadCounters::default().snapshot().io_no_permits, 0);
}
