//! PROPOSED tests. APIs below do not yet exist in maintained ROM.
//! No compilation, RED, or execution is claimed. Install before implementing these APIs.
use rom_ai::{
    AiError, CatalogModel, CatalogSnapshot, CompletionRequest, FailoverPolicy, Message, ModelPrice,
    RouteCursor, RoutingPolicy, RunLimits, UsdNanos, choose, choose_continuation,
};
fn request() -> CompletionRequest {
    CompletionRequest::new(vec![Message::user("synthetic continuation")], 32).unwrap()
}
fn policy() -> RoutingPolicy {
    RoutingPolicy::new(
        1,
        vec!["free-a".into(), "free-b".into()],
        vec!["paid-c".into()],
        Some(ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(10))),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("account")
    .unwrap()
    .with_failover(FailoverPolicy::AdvanceOnConfirmedNonacceptance)
    .unwrap()
}
fn catalog(identity: &str) -> CatalogSnapshot {
    CatalogSnapshot::new(
        identity,
        vec![
            CatalogModel::text("free-a", 4096, true, true, ModelPrice::free()),
            CatalogModel::text("free-b", 4096, true, true, ModelPrice::free()),
            CatalogModel::text(
                "paid-c",
                4096,
                true,
                true,
                ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(10)),
            ),
        ],
    )
    .unwrap()
}
#[test]
fn default_policy_serialization_keeps_legacy_submission_fingerprint_input() {
    let value =
        RoutingPolicy::new(1, vec!["free-a".into()], vec![], None, RunLimits::default()).unwrap();
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        include_str!("fixtures/legacy-policy.json").trim()
    );
    let legacy = serde_json::to_value(&value).unwrap();
    assert!(legacy.get("failover").is_none());
    let restored: RoutingPolicy = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(restored.failover(), FailoverPolicy::SameModel);
    assert_eq!(serde_json::to_value(restored).unwrap(), legacy);
}
#[test]
fn fresh_catalog_does_not_reset_or_relabel_frozen_cursor() {
    let policy = policy();
    let request = request();
    let original = RouteCursor::new(1, "original").unwrap();
    let first = choose(&policy, &catalog("original"), &original, &request).unwrap();
    let forward = first.next_cursor().reject(first.model()).unwrap();
    assert_eq!(
        choose(&policy, &catalog("fresh"), &forward, &request),
        Err(AiError::Conflict)
    );
    let restored: RouteCursor =
        serde_json::from_str(&serde_json::to_string(&forward).unwrap()).unwrap();
    assert_eq!(restored, forward);
    let second = choose_continuation(&policy, &catalog("fresh"), &restored, &request).unwrap();
    assert_eq!(second.model(), "free-b");
    assert_eq!(second.next_cursor().catalog_identity(), "original");
    assert_eq!(second.eligibility_catalog_identity(), Some("fresh"));
    assert!(second.next_cursor().is_rejected("free-a"));
    assert_eq!(
        choose(&policy, &catalog("original"), &original, &request)
            .unwrap()
            .model(),
        "free-a"
    );
}
#[test]
fn paid_continuation_never_returns_to_free_candidates_and_keeps_exact_cap() {
    let policy = policy();
    let request = request();
    let original = RouteCursor::new(1, "original")
        .unwrap()
        .reject("free-a")
        .unwrap()
        .reject("free-b")
        .unwrap();
    let paid = choose_continuation(&policy, &catalog("fresh"), &original, &request).unwrap();
    assert_eq!(paid.model(), "paid-c");
    assert_eq!(paid.maximum_cost(), UsdNanos(10));
    let exhausted = paid.next_cursor().reject("paid-c").unwrap();
    assert_eq!(
        choose_continuation(&policy, &catalog("newer"), &exhausted, &request),
        Err(AiError::ProviderUnavailable)
    );
}
#[test]
fn stale_eligibility_does_not_authorize_now_overpriced_successor() {
    let policy = policy();
    let request = request();
    let cursor = RouteCursor::new(1, "original")
        .unwrap()
        .reject("free-a")
        .unwrap()
        .reject("free-b")
        .unwrap();
    let overpriced = CatalogSnapshot::new(
        "fresh",
        vec![CatalogModel::text(
            "paid-c",
            4096,
            true,
            true,
            ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(11)),
        )],
    )
    .unwrap();
    assert_eq!(
        choose_continuation(&policy, &overpriced, &cursor, &request),
        Err(AiError::ProviderUnavailable)
    );
}
#[test]
fn same_model_default_does_not_expose_new_continuation_selection() {
    let policy = RoutingPolicy::new(
        1,
        vec!["free-a".into(), "free-b".into()],
        vec![],
        None,
        RunLimits::default(),
    )
    .unwrap();
    let cursor = RouteCursor::new(1, "original").unwrap();
    assert_eq!(
        choose_continuation(&policy, &catalog("fresh"), &cursor, &request()),
        Err(AiError::Denied)
    );
}

#[test]
fn legacy_run_record_roundtrip_does_not_add_empty_continuation_state() {
    use rom::Resource;
    let service = rom::Actor::trusted("legacy", "service").with_kind(rom::PrincipalKind::Service);
    let owner = rom::Actor::trusted("legacy", "owner");
    let policy =
        RoutingPolicy::new(1, vec!["free-a".into()], vec![], None, RunLimits::default()).unwrap();
    let run = rom_ai::flow::AiRun::queued(
        "legacy-run",
        &service,
        rom_ai::flow::OwnerIdentity::from_actor(&owner).unwrap(),
        request(),
        policy,
        1000,
    )
    .unwrap();
    let value = run.encode();
    let encoded = value["encoded"].as_str().unwrap();
    let json: serde_json::Value = serde_json::from_str(encoded).unwrap();
    assert_eq!(json["version"], 1);
    assert!(json.get("route_continuations").is_none());
    let record: rom_ai::flow::RunRecord = serde_json::from_str(encoded).unwrap();
    assert_eq!(serde_json::to_string(&record).unwrap(), encoded);
    assert_eq!(
        rom_ai::flow::AiRun::decode(value.clone()).unwrap().encode(),
        value
    );
}
