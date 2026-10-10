//! Execution time is bounded separately from immutable durable attempt identity.
use rom_ai::{
    AiError, AiFuture, AttemptEvidence, CatalogModel, CatalogSnapshot, Completion,
    CompletionRequest, Deadline, ExecutionDeadline, Message, ModelPrice, PreparedAttempt, Provider,
    Reconciliation, RouteCursor, RoutingPolicy, RunLimits, choose,
};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[test]
fn execution_deadline_rejects_empty_or_unbounded_host_time() {
    for duration in [Duration::ZERO, Duration::from_secs(19), Duration::MAX] {
        assert!(matches!(
            ExecutionDeadline::from_remaining(duration),
            Err(AiError::InvalidRequest)
        ));
    }
    let context = ExecutionDeadline::from_remaining(Duration::from_secs(18)).unwrap();
    assert!(context.remaining_ms().unwrap() <= 18_000);
    assert_eq!(format!("{context:?}"), "ExecutionDeadline { .. }");
}

#[test]
fn clones_and_shortening_share_the_original_monotonic_budget() {
    let original = ExecutionDeadline::from_remaining(Duration::from_millis(200)).unwrap();
    let copy = original.clone();
    let shortened = original.shortened(Duration::from_millis(50)).unwrap();
    std::thread::sleep(Duration::from_millis(25));
    assert!(copy.remaining_ms().unwrap() <= 175);
    assert!(shortened.remaining_ms().unwrap() <= 125);
    assert!(matches!(
        original.shortened(Duration::from_millis(200)),
        Err(AiError::DeadlineExceeded)
    ));
}

struct Legacy {
    generation: AtomicUsize,
    lookup: AtomicUsize,
}
impl Provider for Legacy {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async { Err(AiError::UnsupportedCapability) })
    }
    fn complete<'a>(&'a self, _: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async move {
            self.generation.fetch_add(1, Ordering::SeqCst);
            Err(AiError::UnknownOutcome)
        })
    }
    fn reconcile<'a>(&'a self, _: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        Box::pin(async move {
            self.lookup.fetch_add(1, Ordering::SeqCst);
            Ok(Reconciliation::Unresolved)
        })
    }
}
fn prepared() -> PreparedAttempt {
    let request =
        CompletionRequest::new(vec![Message::user("private execution fixture")], 128).unwrap();
    let policy = RoutingPolicy::new(
        1,
        vec!["fixture/model".into()],
        Vec::new(),
        None,
        RunLimits::default(),
    )
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "fixture",
        vec![CatalogModel::text(
            "fixture/model",
            4096,
            true,
            true,
            ModelPrice::free(),
        )],
    )
    .unwrap();
    let route = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "fixture").unwrap(),
        &request,
    )
    .unwrap();
    PreparedAttempt::new(
        "execution:1:1",
        request,
        policy,
        route,
        Deadline::remaining(1000, 1000, 19_000).unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn expired_execution_context_never_invokes_legacy_generation_or_lookup() {
    let provider = Legacy {
        generation: AtomicUsize::new(0),
        lookup: AtomicUsize::new(0),
    };
    let prepared = prepared();
    let frozen = prepared.clone();
    let evidence =
        AttemptEvidence::new(prepared.identity(), None, Some("gen-original".into())).unwrap();
    let context = ExecutionDeadline::from_remaining(Duration::from_millis(1)).unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(matches!(
        provider
            .complete_observed_within(&prepared, context.clone())
            .await,
        Err(AiError::DeadlineExceeded)
    ));
    assert!(matches!(
        provider
            .reconcile_observed_within(&prepared, &evidence, context)
            .await,
        Err(AiError::DeadlineExceeded)
    ));
    assert_eq!(provider.generation.load(Ordering::SeqCst), 0);
    assert_eq!(provider.lookup.load(Ordering::SeqCst), 0);
    assert_eq!(prepared, frozen);
}

#[tokio::test]
async fn compatible_defaults_delegate_without_changing_the_prepared_attempt() {
    let provider = Legacy {
        generation: AtomicUsize::new(0),
        lookup: AtomicUsize::new(0),
    };
    let prepared = prepared();
    let frozen = prepared.clone();
    let evidence = AttemptEvidence::new(prepared.identity(), None, None).unwrap();
    let context = ExecutionDeadline::from_remaining(Duration::from_secs(2)).unwrap();
    assert!(matches!(
        provider
            .complete_observed_within(&prepared, context.clone())
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain {
            cause: AiError::UnknownOutcome,
            ..
        }
    ));
    assert!(matches!(
        provider
            .reconcile_observed_within(&prepared, &evidence, context)
            .await
            .unwrap(),
        rom_ai::ReconciliationObservation::Resolved(Reconciliation::Unresolved)
    ));
    assert_eq!(provider.generation.load(Ordering::SeqCst), 1);
    assert_eq!(provider.lookup.load(Ordering::SeqCst), 1);
    assert_eq!(prepared, frozen);
}
