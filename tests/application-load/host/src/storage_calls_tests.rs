use super::*;
#[test]
fn grouped_and_keyed_operations_have_three_additional_fixed_metric_slots() {
    assert_eq!(StorageCalls::default().snapshot().unwrap().len(), 21);
}
#[test]
fn observed_success_and_failure_preserve_the_actual_result() {
    let calls = StorageCalls::default();
    assert_eq!(calls.measure(Call::Load, || Ok(7)), Ok(7));
    assert_eq!(
        calls.measure::<u8>(Call::Load, || Err(rom::Error::Denied)),
        Err(rom::Error::Denied)
    );
    let counts = calls.snapshot().unwrap()[Call::Load as usize].counts;
    assert_eq!(counts.observed_calls, 2);
    assert_eq!(counts.failed_calls, 1);
    assert_eq!(counts.observer_overflows, 0);
}
#[test]
fn elapsed_overflow_is_reported_instead_of_wrapping_or_fabricating_totals() {
    let calls = StorageCalls::default();
    calls.record(Call::Commit, Some(u64::MAX), false);
    calls.record(Call::Commit, Some(1), false);
    let counts = calls.snapshot().unwrap()[Call::Commit as usize].counts;
    assert_eq!(counts.observed_calls, 1);
    assert_eq!(counts.total_ns, u64::MAX);
    assert_eq!(counts.observer_overflows, 1);
}
#[test]
fn callback_can_reenter_observation_without_a_lock_across_the_operation() {
    let calls = StorageCalls::default();
    assert_eq!(
        calls.measure(Call::Load, || calls.measure(Call::Receipt, || Ok(9))),
        Ok(9)
    );
    assert_eq!(
        calls.snapshot().unwrap()[Call::Load as usize]
            .counts
            .observed_calls,
        1
    );
    assert_eq!(
        calls.snapshot().unwrap()[Call::Receipt as usize]
            .counts
            .observed_calls,
        1
    );
}
