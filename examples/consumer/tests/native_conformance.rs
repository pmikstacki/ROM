#[path = "native_conformance/fixture.rs"]
mod native_fixture;
use native_fixture::Fixture;
use rom_conformance::{FailureCategory, StorageFixture, storage};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn external_fixture_passes_storage_profile_and_releases_owners_before_reopen() {
    let calls = Arc::new(AtomicUsize::new(0));
    storage::basic(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        Fixture::new(false).map(|f| Box::new(f) as Box<dyn StorageFixture>)
    })
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn wrong_profile_fails_before_factory_operations() {
    let calls = AtomicUsize::new(0);
    let error = storage::for_profile(999, || {
        calls.fetch_add(1, Ordering::SeqCst);
        Err(rom::Error::Storage)
    })
    .await
    .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(error.case, "profile.version");
    assert_eq!(error.category, FailureCategory::Profile);
}
#[tokio::test]
async fn broken_fixture_fails_named_counts_assertion() {
    let error =
        storage::basic(|| Fixture::new(true).map(|f| Box::new(f) as Box<dyn StorageFixture>))
            .await
            .unwrap_err();
    assert_eq!(error.case, "storage.bundle.counts");
    assert_eq!(error.category, FailureCategory::Assertion);
}
#[tokio::test]
async fn adapter_errors_are_not_retained_in_conformance_diagnostics() {
    let error = storage::basic(|| Err(rom::Error::Unsupported("SECRET_ADAPTER_ERROR".into())))
        .await
        .unwrap_err();
    assert_eq!(error.case, "storage.factory");
    assert_eq!(error.category, FailureCategory::Adapter);
    assert!(!format!("{error:?} {error}").contains("SECRET_ADAPTER_ERROR"));
}
#[tokio::test]
async fn broken_reopen_fixture_reports_safe_named_failure() {
    let error = storage::basic(|| {
        Fixture::new(false).map(|f| Box::new(f.reject_reopen()) as Box<dyn StorageFixture>)
    })
    .await
    .unwrap_err();
    assert_eq!(error.case, "storage.reopen");
    assert_eq!(error.category, FailureCategory::Adapter);
    assert!(!format!("{error:?} {error}").contains("SECRET_REOPEN_ERROR"));
}
