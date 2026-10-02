use rom_auth_probe_core::*;
use std::collections::BTreeMap;

#[test]
fn embedded_trusted_host_needs_no_adapter_crypto_or_http_dependency() {
    let actor = ActorIssuer::configured_by_host("host", "tenant-a").verified(
        "worker".into(),
        PrincipalKind::Service,
        100,
    );
    let policy = LocalPolicy {
        admins: vec![actor.principal().clone()],
        ..Default::default()
    };
    let mut core = Core::bootstrap(vec![Resource::new(
        "settings",
        "AppSettings",
        BTreeMap::from([("title".into(), Value::Text("before".into()))]),
    )])
    .unwrap();
    assert_eq!(
        core.patch(
            &actor,
            &policy,
            Patch {
                id: "settings",
                expected_revision: 1,
                idempotency: "edit",
                fields: BTreeMap::from([("title".into(), Value::Text("after".into()))])
            },
            0
        )
        .unwrap()
        .revision,
        2
    );
    assert_eq!(
        core.read(&actor, &policy, "settings", &["title".into()], 0)
            .unwrap()["title"],
        Value::Text("after".into())
    );
}

#[test]
fn retry_precondition_is_part_of_canonical_request() {
    let actor = ActorIssuer::configured_by_host("host", "tenant-a").verified(
        "worker".into(),
        PrincipalKind::Service,
        100,
    );
    let policy = LocalPolicy {
        admins: vec![actor.principal().clone()],
        ..Default::default()
    };
    let mut core = Core::bootstrap(vec![Resource::new(
        "settings",
        "AppSettings",
        BTreeMap::from([("title".into(), Value::Text("before".into()))]),
    )])
    .unwrap();
    let fields = BTreeMap::from([("title".into(), Value::Text("after".into()))]);
    core.patch(
        &actor,
        &policy,
        Patch {
            id: "settings",
            expected_revision: 1,
            idempotency: "edit",
            fields: fields.clone(),
        },
        0,
    )
    .unwrap();
    assert_eq!(
        core.patch(
            &actor,
            &policy,
            Patch {
                id: "settings",
                expected_revision: 2,
                idempotency: "edit",
                fields
            },
            0
        ),
        Err("idempotency-conflict")
    );
    assert_eq!(core.event_count(), 1);
}
