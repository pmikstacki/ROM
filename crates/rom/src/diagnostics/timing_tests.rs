use super::checked_elapsed;
use std::time::{Duration, Instant};

#[test]
fn backwards_clock_observation_is_unavailable_instead_of_successful_zero() {
    let start = Instant::now();
    let end = start + Duration::from_nanos(7);
    assert_eq!(checked_elapsed(start, end), Some(7));
    assert_eq!(checked_elapsed(end, start), None);
}
