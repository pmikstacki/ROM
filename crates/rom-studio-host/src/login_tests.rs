use super::login::{Attempt, Attempts};
use super::session::secret;

fn attempt() -> Attempt {
    Attempt {
        provider: "employees".into(),
        browser: secret().unwrap(),
        nonce: secret().unwrap(),
        verifier: secret().unwrap(),
        expires: 120,
        activation: None,
    }
}
#[test]
fn consumed_state_is_one_use_and_browser_and_provider_bound() {
    let attempts = Attempts::new(2);
    let first = attempt();
    let browser = first.browser.clone();
    let state = attempts.insert(first, 100).unwrap();
    assert!(attempts.consume(&state, "other", &browser, 100).is_err());
    assert!(attempts.consume(&state, "employees", "wrong", 100).is_err());
    // A hostile failed consume invalidates this attempt; it cannot be retried by the browser.
    assert!(
        attempts
            .consume(&state, "employees", &browser, 100)
            .is_err()
    );
    let first = attempt();
    let browser = first.browser.clone();
    let state = attempts.insert(first, 100).unwrap();
    assert!(attempts.consume(&state, "employees", &browser, 100).is_ok());
    assert!(
        attempts
            .consume(&state, "employees", &browser, 100)
            .is_err()
    );
}
#[test]
fn finite_attempt_admission_recovers_capacity_after_expiry() {
    let attempts = Attempts::new(1);
    let first = attempt();
    let browser = first.browser.clone();
    let state = attempts.insert(first, 100).unwrap();
    assert!(attempts.insert(attempt(), 100).is_err());
    assert!(
        attempts
            .consume(&state, "employees", &browser, 120)
            .is_err()
    );
    let mut future = attempt();
    future.expires = 200;
    assert!(attempts.insert(future, 120).is_ok());
    attempts.close();
    assert!(attempts.insert(attempt(), 120).is_err());
}
