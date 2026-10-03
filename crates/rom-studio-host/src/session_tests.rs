use super::session::{SessionEvidence, SessionStore};
use rom::Actor;

fn evidence(expiry: u64) -> SessionEvidence {
    SessionEvidence {
        actor: Actor::trusted("fixture", "alice").expires_at(expiry),
        user_id: "alice-user".into(),
        token_expiry: expiry,
        credentials: None,
    }
}

#[tokio::test]
async fn logout_cancels_all_session_observers_without_revoking_other_sessions() {
    let store = SessionStore::new(2, 60);
    let first = store.insert(evidence(150), 100).unwrap();
    let second = store.insert(evidence(150), 100).unwrap();
    let mut tab_a = first.cancellation();
    let mut tab_b = first.cancellation();
    let other = second.cancellation();
    store.remove(first.cookie());
    tab_a.changed().await.unwrap();
    tab_b.changed().await.unwrap();
    assert!(*tab_a.borrow());
    assert!(*tab_b.borrow());
    assert!(!*other.borrow());
    assert!(store.lookup(first.cookie(), 100).is_none());
    assert!(store.lookup(second.cookie(), 100).is_some());
}

#[test]
fn expired_sessions_release_capacity_and_never_extend_identity_expiry() {
    let store = SessionStore::new(1, 60);
    let first = store.insert(evidence(105), 100).unwrap();
    assert!(store.insert(evidence(150), 100).is_err());
    assert!(store.lookup(first.cookie(), 104).is_some());
    assert!(store.lookup(first.cookie(), 105).is_none());
    assert!(*first.cancellation().borrow());
    assert!(store.insert(evidence(150), 105).is_ok());
    let short = SessionStore::new(1, 10).insert(evidence(200), 100).unwrap();
    assert_eq!(short.expires_at(), 110);
}

#[test]
fn cookie_and_csrf_secrets_are_independent_and_opaque() {
    let store = SessionStore::new(2, 60);
    let first = store.insert(evidence(150), 100).unwrap();
    let second = store.insert(evidence(150), 100).unwrap();
    assert_ne!(first.cookie(), second.cookie());
    assert_ne!(first.cookie(), first.csrf());
    assert_ne!(first.generation(), second.generation());
    assert_eq!(first.cookie().len(), 43);
    assert!(!first.cookie().contains("alice"));
    assert!(first.check_csrf(first.csrf()));
    assert!(!first.check_csrf(second.csrf()));
    assert!(!first.check_csrf(""));
}

#[test]
fn shutdown_closes_existing_sessions_and_prevents_new_admission() {
    let store = SessionStore::new(1, 60);
    let first = store.insert(evidence(150), 100).unwrap();
    store.close();
    assert!(*first.cancellation().borrow());
    assert!(store.lookup(first.cookie(), 100).is_none());
    assert!(store.insert(evidence(150), 100).is_err());
}

#[test]
fn transient_admission_failure_does_not_log_out_a_valid_session() {
    let store = SessionStore::new(1, 60);
    let session = store.insert(evidence(150), 100).unwrap();
    store.failed(session.cookie(), &rom::Error::Overloaded);
    assert!(store.lookup(session.cookie(), 100).is_some());
    assert!(!*session.cancellation().borrow());
    store.failed(session.cookie(), &rom::Error::Denied);
    assert!(store.lookup(session.cookie(), 100).is_none());
    assert!(*session.cancellation().borrow());
}
